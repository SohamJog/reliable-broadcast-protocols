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

![latency_boxplot_88](https://github.com/user-attachments/assets/a90c72c6-c9e1-4d39-ac95-431b113a410e)



Final Results:
Here are all the results we got after running each of the following configurations 4 times:

| Protocol_Config          | Mean (ms) | Median (ms) | Std Dev (ms) |
|---------------------------|-----------|-------------|-------------|
| addrbc_false @ 4 nodes    | 2.55      | 2.00        | 0.96        |
| addrbc_true @ 4 nodes     | 3.08      | 2.50        | 1.72        |
| rbc_false @ 4 nodes       | 2.08      | 2.00        | 0.72        |
| rbc_true @ 4 nodes        | 3.52      | 2.50        | 2.75        |
| ctrbc_false @ 4 nodes     | 2.44      | 2.00        | 1.05        |
| ctrbc_true @ 4 nodes      | 1.54      | 1.25        | 0.62        |
| addrbc_false @ 16 nodes   | 31.23     | 30.82       | 8.21        |
| addrbc_true @ 16 nodes    | 45.57     | 42.88       | 16.19       |
| rbc_false @ 16 nodes      | 41.19     | 41.47       | 10.12       |
| rbc_true @ 16 nodes       | 49.38     | 47.75       | 12.54       |
| ctrbc_false @ 16 nodes    | 30.51     | 30.28       | 4.66        |
| ctrbc_true @ 16 nodes     | 44.75     | 35.56       | 29.78       |
| addrbc_false @ 40 nodes   | 471.92    | 413.88      | 210.36      |
| addrbc_true @ 40 nodes    | 1348.30   | 1472.97     | 708.87      |
| rbc_false @ 40 nodes      | 728.26    | 581.67      | 278.38      |
| rbc_true @ 40 nodes       | 1438.52   | 1566.81     | 451.42      |
| ctrbc_false @ 40 nodes    | 1159.04   | 1301.62     | 418.22      |
| ctrbc_true @ 40 nodes     | 1443.83   | 1617.53     | 361.36      |
| addrbc_false @ 64 nodes   | 2210.47   | 1911.39     | 803.29      |
| addrbc_true @ 64 nodes    | 10770.80  | 12593.22    | 6100.78     |
| rbc_false @ 64 nodes      | 4780.82   | 4953.31     | 2293.92     |
| rbc_true @ 64 nodes       | 6086.93   | 6952.49     | 1688.17     |
| ctrbc_false @ 64 nodes    | 5161.27   | 5869.44     | 1557.52     |
| ctrbc_true @ 64 nodes     | 5988.61   | 6822.36     | 1564.09     |
| addrbc_false @ 88 nodes   | 6729.96   | 7772.42     | 1823.07     |
| addrbc_true @ 88 nodes    | 25075.71  | 29047.53    | 17107.47    |
| rbc_false @ 88 nodes      | 13091.11  | 16040.39    | 5188.66     |
| rbc_true @ 88 nodes       | 15623.01  | 17954.26    | 3827.69     |
| ctrbc_false @ 88 nodes    | 13738.49  | 15861.78    | 4360.09     |
| ctrbc_true @ 88 nodes     | 17232.74  | 18159.90    | 5265.91     |
