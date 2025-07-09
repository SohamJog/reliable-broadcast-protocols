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

See the `benchmark` folder for additional information on how to run benchmarks and reproduce the results in the paper
