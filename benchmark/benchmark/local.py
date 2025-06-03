# Copyright(C) Facebook, Inc. and its affiliates.
import subprocess
from math import ceil
from os.path import basename, splitext
from time import sleep

from benchmark.commands import CommandMaker
from benchmark.config import Key, LocalCommittee, NodeParameters, BenchParameters, ConfigError
from benchmark.logs import LogParser, ParseError
from benchmark.utils import Print, BenchError, PathMaker


class LocalBench:
    BASE_PORT = 9000
    cl_bport = 10000
    def __init__(self, bench_parameters_dict, node_parameters_dict):

        try:
            self.bench_parameters = BenchParameters(bench_parameters_dict)
            self.node_parameters = NodeParameters(node_parameters_dict)
        except ConfigError as e:
            raise BenchError('Invalid nodes or bench parameters', e)

    def __getattr__(self, attr):
        return getattr(self.bench_parameters, attr)

    def _background_run(self, command, log_file):
        name = splitext(basename(log_file))[0]
        cmd = f'{command} > {log_file}'
        print("Command running: {}",command)
        #print(log_file)
        try:
            subprocess.run(['tmux', 'new', '-d', '-s', name, cmd], check=True)
        except subprocess.SubprocessError as e:
            raise BenchError('Failed to kill testbed', e)
        

    def _kill_nodes(self):
        try:
            cmd = CommandMaker.kill().split()
            subprocess.run(cmd, stderr=subprocess.DEVNULL)
        except subprocess.SubprocessError as e:
            raise BenchError('Failed to kill testbed', e)

    def run(self, debug=False):
        assert isinstance(debug, bool)
        Print.heading('Starting local benchmark')

        # Kill any previous testbed.
        self._kill_nodes()

        try:
            Print.info('Setting up testbed...')
            nodes, rate = self.nodes[0], self.rate[0]

            # Cleanup all files.
            cmd = f'{CommandMaker.clean_logs()} ; {CommandMaker.cleanup()}'
            subprocess.run([cmd], shell=True, stderr=subprocess.DEVNULL)
            sleep(0.5)  # Removing the store may take time.

            # Recompile the latest code.
            cmd = CommandMaker.compile().split()
            ret_code = subprocess.run(cmd, check=True, cwd=PathMaker.node_crate_path())
            #Print.info(ret_code)
            # Create alias for the client and nodes binary.
            cmd = CommandMaker.alias_binaries(PathMaker.binary_path())
            subprocess.run([cmd], shell=True)

            # Generate configuration files.
            # keys = []
            # key_files = [PathMaker.key_file(i) for i in range(nodes)]
            # for filename in key_files:
            #     cmd = CommandMaker.generate_key(filename).split()
            #     subprocess.run(cmd, check=True)
            #     keys += [Key.from_file(filename)]

            names = [str(x) for x in range(nodes)]
            ip_file = ""
            for x in range(nodes):
                port = self.BASE_PORT + x
                ip_file += '127.0.0.1:'+ str(port) + "\n"
            with open("ip_file", 'w') as f:
                f.write(ip_file)
            f.close()
            #committee = LocalCommittee(names, self.BASE_PORT)
            #ip_file.print("ip_file")

            # Generate the configuration files for add-rbc
            cmd = CommandMaker.generate_config_files(self.BASE_PORT,self.cl_bport,9500,nodes)
            self._background_run(cmd,"err.log")


            # # Run the primaries (except the faulty ones).
            for i in range(nodes):
                cmd = CommandMaker.run_primary(
                PathMaker.key_file(i),
                self.protocol,
                self.bfile,
                self.byzantine,
                self.crash,
                debug=debug
                )
                log_file = PathMaker.primary_log_file(i)
                self._background_run(cmd, log_file)
        except (subprocess.SubprocessError, ParseError) as e:
            self._kill_nodes()
            raise BenchError('Failed to run benchmark', e)
