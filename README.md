# add-rbc
A Rust implementation of Reliable Broadcast based on Asynchronous Data Dissemination (ADD) as described [here](https://eprint.iacr.org/2021/777.pdf)

---
## Directory Structure

This directory contains implementations of various reliable broadcast algorithms to compare with ADD-RBC. You will find the protocols in the `consensus` directory. 

## Scripts

Initialize:
```bash
cargo build --release
mkdir logs
./scripts/create_testdata.sh <num_nodes>
```

Test regular(Bracha's) RBC with 16 nodes:
```bash
pkill -f "./target/release/node" 
./scripts/test.sh testdata/hyb_16/syncer Hi false testdata/test_msgs.txt rbc 16
```

To try ADD-RBC or other protocols, you can append the protocol name to the test script:
```bash
./scripts/test.sh testdata/hyb_16/syncer Hi false testdata/test_msgs.txt addrbc 16
```

Run this script to check if the logs are consistent:
```bash
 ./scripts/check_logs.sh <number of nodes>
```

Test multiple runs of ADD-RBC: 
```bash
./scripts/multiple_runs.sh <num_iterations> [<num_nodes> <protocol> <byzantine>]
```

---

## Benchmarks

To run the benchmarks, run the following script:
```bash
./scripts/run_benchmarks.sh
```
This will run 4 experiments on ADD-RBC, CTRBC, Bracha's RBC on 4, 16, 40, 64, and 88 nodes, with and without (n-1)/3 byzantine nodes. It will then store all the results in `/bench_logs`

To aggregate the data from benchmarks and generate boxplots and histograms:
```bash
 python scripts/aggregate_and_plot.py
```
This will store all the graphs in `results`

---

## Experiments
We ran 4 experiments on ADD-RBC, CTRBC, Bracha's RBC on 4, 16, 40, 64, and 88 nodes, with and without (n-1)/3 byzantine nodes.

We found out that ADD RBC is significantly faster on environments without Byzantine nodes, while CTRBC is relatively fastest in environments with Byzantine nodes. 
Here is an example comparing all the protocols with and without byzantine nodes on n = 88
/Users/sohamjog/Desktop/research/add-rbc/results/latency_boxplot_88.png

Final Results:
TODO
