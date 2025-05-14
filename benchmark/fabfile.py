# Copyright(C) Facebook, Inc. and its affiliates.
from fabric import task

from benchmark.local import LocalBench
from benchmark.logs import ParseError, LogParser
from benchmark.utils import Print
from benchmark.plot import Ploter, PlotError
from benchmark.instance import InstanceManager
from benchmark.remote import Bench, BenchError
from benchmark.utils import PathMaker

# === Global Benchmark Parameters ===
DEFAULT_BENCH_PARAMS = {
    'faults': 0,
    'nodes': [16],
    'workers': 1,
    'collocate': True,
    'rate': [10_000, 110_000],
    'tx_size': 512,
    'duration': 300,
    'runs': 2,
    'protocol': 'addrbc',
    'bfile': 'longer_test_msgs.txt',
    'byzantine': False,
}

DEFAULT_NODE_PARAMS = {
    'header_size': 1_000,
    'max_header_delay': 200,
    'gc_depth': 50,
    'sync_retry_delay': 10_000,
    'sync_retry_nodes': 3,
    'batch_size': 500_000,
    'max_batch_delay': 200,
}


# === Tasks ===

@task
def local(ctx, debug=True):
    ''' Run benchmarks on localhost '''
    try:
        ret = LocalBench(DEFAULT_BENCH_PARAMS, DEFAULT_NODE_PARAMS).run(debug)
    except BenchError as e:
        Print.error(e)


@task
def log_v(ctx, debug=True):
    ''' Parse local logs '''
    try:
        Print.info('Parsing logs...')
        result = LogParser.process(PathMaker.logs_path(), faults=DEFAULT_BENCH_PARAMS['faults'])
        print(result.result())
    except BenchError as e:
        Print.error(e)


# The parameter nodes determines how many instances to create in each AWS region. That is, if you specified 5 AWS regions as in the example of step 3, setting nodes=2 will creates a total of 16 machines:

@task
def create(ctx, nodes=2):
    ''' Create a testbed '''
    try:
        InstanceManager.make().create_instances(nodes)
    except BenchError as e:
        Print.error(e)


@task
def destroy(ctx):
    ''' Destroy the testbed '''
    try:
        InstanceManager.make().terminate_instances()
    except BenchError as e:
        Print.error(e)


@task
def start(ctx, max=2):
    ''' Start machines '''
    try:
        InstanceManager.make().start_instances(max)
    except BenchError as e:
        Print.error(e)


@task
def stop(ctx):
    ''' Stop all machines '''
    try:
        InstanceManager.make().stop_instances()
    except BenchError as e:
        Print.error(e)


@task
def info(ctx):
    ''' Info about machines '''
    try:
        InstanceManager.make().print_info()
    except BenchError as e:
        Print.error(e)


@task
def install(ctx):
    ''' Install codebase on machines '''
    try:
        Bench(ctx).install()
    except BenchError as e:
        Print.error(e)


@task
def remote(ctx, debug=False):
    ''' Run benchmarks on AWS '''
    try:
        Bench(ctx).run(
            DEFAULT_BENCH_PARAMS, DEFAULT_NODE_PARAMS,
            # DEFAULT_BENCH_PARAMS['protocol'],
            # DEFAULT_BENCH_PARAMS['bfile'],
            # DEFAULT_BENCH_PARAMS['byzantine'],
            debug
        )
    except BenchError as e:
        Print.error(e)


@task
def rerun(ctx, debug=False):
    ''' Re-run benchmarks without full re-setup '''
    try:
        Bench(ctx).justrun(
            DEFAULT_BENCH_PARAMS, DEFAULT_NODE_PARAMS,
            DEFAULT_BENCH_PARAMS['protocol'],
            DEFAULT_BENCH_PARAMS['bfile'],
            DEFAULT_BENCH_PARAMS['byzantine'],
            debug
        )
    except BenchError as e:
        Print.error(e)


@task
def plot(ctx):
    ''' Plot performance from logs '''
    plot_params = {
        'faults': [0],
        'nodes': [10, 20, 50],
        'workers': [1],
        'collocate': True,
        'tx_size': 512,
        'max_latency': [3_500, 4_500],
    }
    try:
        Ploter.plot(plot_params)
    except PlotError as e:
        Print.error(BenchError('Failed to plot performance', e))


@task
def kill(ctx):
    ''' Kill all processes '''
    try:
        Bench(ctx).kill()
    except BenchError as e:
        Print.error(e)


@task
def logs(ctx):
    ''' Download and print logs '''
    try:
        print(Bench(ctx).pull_logs(
            DEFAULT_BENCH_PARAMS, DEFAULT_NODE_PARAMS,
            # DEFAULT_BENCH_PARAMS['protocol'],
            # DEFAULT_BENCH_PARAMS['bfile'],
            # DEFAULT_BENCH_PARAMS['byzantine']
        ))
    except ParseError as e:
        Print.error(BenchError('Failed to parse logs', e))
