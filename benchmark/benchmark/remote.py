# Copyright(C) Facebook, Inc. and its affiliates.
from collections import OrderedDict
from fabric import Connection, ThreadingGroup as Group
from fabric.exceptions import GroupException
from paramiko import RSAKey
from paramiko.ssh_exception import PasswordRequiredException, SSHException
from os.path import basename, splitext
from time import sleep
from math import ceil
from copy import deepcopy
import subprocess
import time

from benchmark.config import Committee, Key, NodeParameters, BenchParameters, ConfigError
from benchmark.utils import BenchError, Print, PathMaker, progress_bar
from benchmark.commands import CommandMaker
from benchmark.logs import LogParser, ParseError
from benchmark.instance import InstanceManager


class FabricError(Exception):
    ''' Wrapper for Fabric exception with a meaningfull error message. '''

    def __init__(self, error):
        assert isinstance(error, GroupException)
        message = list(error.result.values())[-1]
        super().__init__(message)


class ExecutionError(Exception):
    pass


class Bench:
    def __init__(self, ctx):
        self.manager = InstanceManager.make()
        self.settings = self.manager.settings
        try:
            ctx.connect_kwargs.pkey = RSAKey.from_private_key_file(
                self.manager.settings.key_path
            )
            self.connect = ctx.connect_kwargs
        except (IOError, PasswordRequiredException, SSHException) as e:
            raise BenchError('Failed to load SSH key', e)

    def _check_stderr(self, output):
        if isinstance(output, dict):
            for x in output.values():
                if x.stderr:
                    raise ExecutionError(x.stderr)
        else:
            if output.stderr:
                raise ExecutionError(output.stderr)

    def install(self):
        Print.info('Installing rust and cloning the repo...')
        cmd = [
            'if command -v apt-get &>/dev/null; then '
                'sudo apt-get update && '
                'sudo apt-get -y upgrade && '
                'sudo apt-get -y autoremove && '
                'sudo apt-get -y install build-essential cmake libgmp-dev clang tmux; '
            'else '
                'sudo yum update -y && '
                'sudo yum install -y gcc gcc-c++ make cmake git curl clang gmp-devel tmux; '
            'fi',
            'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y',
            'source $HOME/.cargo/env',
            'rustup install 1.83.0',
            'rustup override set 1.83.0',
            f'(git clone {self.settings.repo_url} || (cd {self.settings.repo_name} ; git pull))'
        ]
 
        hosts = self.manager.hosts(flat=True)
        try:
            g = Group(*hosts, user='ec2-user', connect_kwargs=self.connect)
            g.run(' && '.join(cmd), hide=True)
            Print.heading(f'Initialized testbed of {len(hosts)} nodes')
        except (GroupException, ExecutionError) as e:
            e = FabricError(e) if isinstance(e, GroupException) else e
            raise BenchError('Failed to install repo on testbed', e)
 

    def kill(self, hosts=[], delete_logs=False):
        assert isinstance(hosts, list)
        assert isinstance(delete_logs, bool)
        hosts = hosts if hosts else self.manager.hosts(flat=True)
        delete_logs = CommandMaker.clean_logs() if delete_logs else 'true'
        cmd = [delete_logs, f'({CommandMaker.kill()} || true)']
        try:
            g = Group(*hosts, user='ec2-user', connect_kwargs=self.connect)
            g.run(' && '.join(cmd), hide=True)
        except GroupException as e:
            raise BenchError('Failed to kill nodes', FabricError(e))

    def _select_hosts(self, bench_parameters):
        # Collocate the primary and its workers on the same machine.
        if bench_parameters.collocate:
            nodes = max(bench_parameters.nodes)

            # Ensure there are enough hosts.
            hosts = self.manager.hosts()
            print("{} {}",sum(len(x) for x in hosts.values()), nodes)
            if sum(len(x) for x in hosts.values()) < nodes:
                return []

            # Select the hosts in different data centers.
            ordered = zip(*hosts.values())
            ordered = [x for y in ordered for x in y]
            return ordered[:nodes]

        # Spawn the primary and each worker on a different machine. Each
        # authority runs in a single data center.
        else:
            primaries = max(bench_parameters.nodes)

            # Ensure there are enough hosts.
            hosts = self.manager.hosts()
            if len(hosts.keys()) < primaries:
                return []
            for ips in hosts.values():
                if len(ips) < bench_parameters.workers + 1:
                    return []

            # Ensure the primary and its workers are in the same region.
            selected = []
            for region in list(hosts.keys())[:primaries]:
                ips = list(hosts[region])[:bench_parameters.workers + 1]
                selected.append(ips)
            return selected

    def _background_run(self, host, command, log_file):
        name = splitext(basename(log_file))[0]
        cmd = f'tmux new -d -s "{name}" "{command} |& tee {log_file}"'
        c = Connection(host, user='ec2-user', connect_kwargs=self.connect)
        output = c.run(cmd, hide=True)
        self._check_stderr(output)

    def _update(self, hosts, collocate):
        if collocate:
            ips = list(set(hosts))
        else:
            ips = list(set([x for y in hosts for x in y]))

        Print.info(
            f'Updating {len(ips)} machines (branch "{self.settings.branch}")...'
        )
        cmd = [
            f'(cd {self.settings.repo_name} && git fetch -f)',
            f'(cd {self.settings.repo_name} && git checkout -f {self.settings.branch})',
            f'(cd {self.settings.repo_name} && git pull -f)',
            'source $HOME/.cargo/env',
            'sudo yum install -y pkgconfig openssl-devel',
            # 'sudo apt install pkg-config && sudo apt install libssl-dev',
            'export RUSTFLAGS="-C target-feature=+aes,+ssse3"',
            f'(cd {self.settings.repo_name} && {CommandMaker.compile()})',
            CommandMaker.alias_binaries(
                f'./{self.settings.repo_name}/target/release/'
            )
        ]
        g = Group(*ips, user='ec2-user', connect_kwargs=self.connect)
        print(g.run(' && '.join(cmd), hide=True))

    def _config(self, hosts, node_parameters, bench_parameters):
        Print.info('Generating configuration files...')
        #print(hosts)
        # Cleanup all local configuration files.
        cmd = CommandMaker.cleanup()
        subprocess.run([cmd], shell=True, stderr=subprocess.DEVNULL)

        # Recompile the latest code.
        cmd = CommandMaker.compile().split()
        print("Running command ",cmd)
        subprocess.run(cmd, check=True, cwd=PathMaker.node_crate_path())

        # Create alias for the client and nodes binary.
        cmd = CommandMaker.alias_binaries(PathMaker.binary_path())
        subprocess.run([cmd], shell=True)

        # Generate configuration files.
        # keys = []
        # key_files = [PathMaker.key_file(i) for i in range(len(hosts))]
        # for filename in key_files:
        #     cmd = CommandMaker.generate_key(filename).split()
        #     subprocess.run(cmd, check=True)
        #     keys += [Key.from_file(filename)]
        #committee = LocalCommittee(names, self.BASE_PORT)
        #ip_file.print("ip_file")

        # Generate the configuration files for add-rbc
        cmd = CommandMaker.generate_config_files(self.settings.base_port,self.settings.client_base_port,self.settings.client_run_port,len(hosts))
        subprocess.run(cmd,shell=True)
        names = [str(x) for x in range(len(hosts))]
        ip_file = ""
        syncer=""
        for x in range(len(hosts)):
            port = self.settings.base_port + x
            syncer_port = self.settings.client_base_port + x
            ip_file += hosts[x]+ ":"+ str(port) + "\n"
            syncer += hosts[x] + ":" + str(syncer_port) + "\n"
        ip_file += hosts[0] + ":" + str(self.settings.client_run_port) + "\n"
        with open("ip_file", 'w') as f:
            f.write(ip_file)
        f.close()
        with open("syncer",'w') as f:
            f.write(syncer)
        f.close()
        #names = [str(x) for x in range(len(hosts))]

        if bench_parameters.collocate:
            workers = bench_parameters.workers
            addresses = OrderedDict(
                (x, [y] * (workers + 1)) for x, y in zip(names, hosts)
            )
        else:
            addresses = OrderedDict(
                (x, y) for x, y in zip(names, hosts)
            )
        committee = Committee(addresses, self.settings.base_port)
        committee.print(PathMaker.committee_file())

        node_parameters.print(PathMaker.parameters_file())
        # start the syncer on the first node first. 

        # Cleanup all nodes and upload configuration files.
        names = names[:len(names)-bench_parameters.faults]
        progress = progress_bar(names, prefix='Uploading config files:')
        for i, name in enumerate(progress):
            c = Connection(hosts[i], user='ec2-user', connect_kwargs=self.connect)
            c.run(f'{CommandMaker.cleanup()} || true', hide=True)
            if i == 0:
                print('Node 0: writing syncer')
                c.put(PathMaker.syncer(),'.')
            c.put(PathMaker.key_file(i), '.')
            c.put(PathMaker.t_key_file(),'.')
            c.put(PathMaker.t_testdata_file(),'.')

            c.put("ip_file",'.')
            #c.put(PathMaker.parameters_file(), '.')
        Print.info('Booting primaries...')
        st_time = round(time.time() * 1000) + 60000
        batches = 1
        per_batch = 2000
        exp_vals = self.exp_setup(4)
        import numpy as np
        tri = np.max(exp_vals) - np.min(exp_vals)
        for i, ip in enumerate(hosts):
            if i == 0:
                # Run syncer first
                print('Running syncer')
                sync_cmd = CommandMaker.run_syncer(PathMaker.key_file(i), PathMaker.t_testdata_file(), self.byzantine)
                print(sync_cmd)
                sync_log = PathMaker.syncer_log_file()
                self._background_run(ip, sync_cmd, sync_log)

            # Run primary on all nodes
            unzip_cmd = CommandMaker.unzip_tkeys('data.tar.gz')
            print(unzip_cmd)
            self._background_run(ip, unzip_cmd, "unzip.log")

            cmd = CommandMaker.run_primary(
                PathMaker.key_file(i),
                self.protocol,
                self.bfile,
                self.byzantine,
                self.crash,
            )
            print(cmd)
            log_file = PathMaker.primary_log_file(i)
            self._background_run(ip, cmd, log_file)

       
        return committee

    def exp_setup(self,n):
        import numpy as np
        values = np.random.normal(loc=540000,scale=10000,size=n)
        arr_int = []
        for val in values:
            arr_int.append(int(val))
        return arr_int


    # def _just_run(self, hosts, node_parameters, bench_parameters):

    #     Print.info('Booting primaries...')
    #     st_time = round(time.time() * 1000) + 60000
    #     batches = 1
    #     per_batch = 2000

    #     for i,ip in enumerate(hosts):
    #         #host = Committee.ip(address)
    #         if i == 0:
    #             # Run syncer first
    #             print('Running syncer')
    #             cmd = CommandMaker.run_syncer(
    #                 PathMaker.key_file(i),
    #             )
    #             print(cmd)
    #             log_file = PathMaker.syncer_log_file()
    #             self._background_run(ip, cmd, log_file)
    #         cmd = CommandMaker.run_primary(
    #             PathMaker.key_file(i),
    #             self.protocol,
    #             self.bfile,
    #             self.byzantine,
    #             self.crash
    #         )
    #         log_file = PathMaker.primary_log_file(i)
    #         self._background_run(ip, cmd, log_file)
    def _just_run(self, hosts, node_parameters, bench_parameters):
        Print.info('Booting primaries...')
        names = [str(x) for x in range(len(hosts))]

        for i, ip in enumerate(hosts):
            if i == 0:
                # Run syncer
                print('Running syncer')
                cmd = CommandMaker.run_syncer(
                    PathMaker.key_file(i),
                    PathMaker.t_testdata_file(),
                    self.byzantine
                )
                print(cmd)
                log_file = PathMaker.syncer_log_file()
                self._background_run(ip, cmd, log_file)

            # Optionally unzip tkeys/tdata if not guaranteed to exist
            unzip_cmd = CommandMaker.unzip_tkeys("data.tar.gz")
            print(unzip_cmd)
            self._background_run(ip, unzip_cmd, "unzip.log")

            # Run primary
            cmd = CommandMaker.run_primary(
                PathMaker.key_file(i),
                self.protocol,
                self.bfile,
                self.byzantine,
                self.crash
            )
            print(cmd)
            log_file = PathMaker.primary_log_file(i)
            self._background_run(ip, cmd, log_file)


    def _logs(self, hosts, faults):
        # Delete local logs (if any).
        # cmd = CommandMaker.clean_logs()
        # subprocess.run([cmd], shell=True, stderr=subprocess.DEVNULL)

        # Download log files.
        #workers_addresses = committee.workers_addresses(faults)
        progress = progress_bar(hosts, prefix='Downloading workers logs:')
        # TODO: only get syncer
        for i, address in enumerate(progress):
            if i == 0:
                c = Connection(address, user='ec2-user', connect_kwargs=self.connect)
                remote_path = PathMaker.syncer_log_file()
                local_path = PathMaker.syncer_log_file()
                print(f"Fetching syncer log from {address}")
                print(f"Remote path: {remote_path}")
                print(f"Local path: {local_path}")
                c.get(remote_path, local=local_path)

            # try:
            #     c.get(
            #     PathMaker.client_log_file(i, 0), 
            #     local=PathMaker.client_log_file(i, 0)
            #     )
            # except Exception as e: 
            #     print(f"Failed to fetch client log from {address}: { PathMaker.client_log_file(i, 0)} — {e}")



        # Parse logs and return the parser.
        # Print.info('Parsing logs and computing performance...')
        # return LogParser.process(PathMaker.logs_path(), faults=faults)

    def run(self, bench_parameters_dict, node_parameters_dict, debug=False):
        assert isinstance(debug, bool)
        Print.heading('Starting remote benchmark')
        try:
            bench_parameters = BenchParameters(bench_parameters_dict)
            node_parameters = NodeParameters(node_parameters_dict)
        except ConfigError as e:
            raise BenchError('Invalid nodes or bench parameters', e)

        self.protocol = bench_parameters.protocol
        self.bfile = bench_parameters.bfile
        self.byzantine = bench_parameters.byzantine
        self.crash = bench_parameters.crash
        # Select which hosts to use.
        selected_hosts = self._select_hosts(bench_parameters)
        print(selected_hosts)
        if not selected_hosts:
            Print.warn('There are not enough instances available')
            return

        # Update nodes.
        try:
            self._update(selected_hosts, bench_parameters.collocate)
        except (GroupException, ExecutionError) as e:
            e = FabricError(e) if isinstance(e, GroupException) else e
            raise BenchError('Failed to update nodes', e)

        # Upload all configuration files.
        try:
            committee = self._config(
                selected_hosts, node_parameters, bench_parameters
            )
        except (subprocess.SubprocessError, GroupException) as e:
            e = FabricError(e) if isinstance(e, GroupException) else e
            raise BenchError('Failed to configure nodes', e)


    def justrun(self, bench_parameters_dict, node_parameters_dict, debug=False):
        assert isinstance(debug, bool)
        Print.heading('Starting remote benchmark')
        try:
            bench_parameters = BenchParameters(bench_parameters_dict)
            self.protocol = bench_parameters.protocol
            self.bfile = bench_parameters.bfile
            self.byzantine = bench_parameters.byzantine
            self.crash = bench_parameters.crash
            node_parameters = NodeParameters(node_parameters_dict)
        except ConfigError as e:
            raise BenchError('Invalid nodes or bench parameters', e)

        # Select which hosts to use.
        selected_hosts = self._select_hosts(bench_parameters)
        print(selected_hosts)
        if not selected_hosts:
            Print.warn('There are not enough instances available')
            return

        # Update nodes.
        try:
            self._update(selected_hosts, bench_parameters.collocate)
        except (GroupException, ExecutionError) as e:
            e = FabricError(e) if isinstance(e, GroupException) else e
            raise BenchError('Failed to update nodes', e)

        # Upload all configuration files.
        try:
            committee = self._just_run(
                selected_hosts, node_parameters, bench_parameters
            )
        except (subprocess.SubprocessError, GroupException) as e:
            e = FabricError(e) if isinstance(e, GroupException) else e
            raise BenchError('Failed to configure nodes', e)

    def pull_logs(self, bench_parameters_dict, node_parameters_dict, debug=False):
        import re
        from statistics import mean
        from collections import defaultdict
        from fabric import Connection

        Print.heading('Fetching latency logs...')
        try:
            bench_parameters = BenchParameters(bench_parameters_dict)
            node_parameters = NodeParameters(node_parameters_dict)
        except ConfigError as e:
            raise BenchError('Invalid nodes or bench parameters', e)

        selected_hosts = self._select_hosts(bench_parameters)
        if not selected_hosts:
            raise BenchError('No hosts available', None)

        host = selected_hosts[0] if bench_parameters.collocate else selected_hosts[0][0]
        c = Connection(host, user='ec2-user', connect_kwargs=self.connect)

        Print.info(f'Running latency script on: {host}')
        result = c.run(f'./reliable-broadcast-protocols/benchmark/latencies.sh {max(bench_parameters.nodes)}', hide=True)
        output = result.stdout

        # Parse latency output
        pattern = r'ID (\d+)\s+\|\s+(\d+)\s+bytes\s+\|\s+([\d.]+)\s+\|\s+\d+ latencies'
        matches = re.findall(pattern, output)

        latency_data = defaultdict(list)
        seen_ids = []

        for msg_id, byte_size, latency in matches:
            latency_data[int(byte_size)].append(float(latency))
            seen_ids.append(int(msg_id))

        #validate
        aggregated = {k: round(mean(v), 3) for k, v in latency_data.items()}

        missing_ids = []
        num_nodes = max(bench_parameters.nodes)
        for i in range(num_nodes):
            base = i * 10000
            for j in range(1, 7):
                if base + j not in seen_ids:
                    missing_ids.append(base + j)

        if missing_ids:
            Print.warn(f"Not all message IDs found. Missing: {missing_ids}")
        Print.heading('Average Latencies by Message Size')
        for byte_size in sorted(aggregated):
            print(f"{byte_size} bytes: {aggregated[byte_size]} ms")

        return aggregated
 
