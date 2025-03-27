# add-rbc
A rust implementation of Reliable Broadcast based on Asynchronous Data Dissemination (ADD) as described [here](https://eprint.iacr.org/2021/777.pdf)

---
## Directory Structure

This directory contains implementations of various reliable broadcast algorithms to compare with ADD-RBC. You will find the protocols in the `consensus` directory. 

## Scripts

Initialize:
```bash
cargo build --release
mkdir logs
cd testdata
mkdir hyb_16
./target/release/genconfig --NumNodes 16 --delay 10 --blocksize 100 --client_base_port 7000 --target testdata/hyb_16/ --payload 100 --out_type json --base_port 9000 --client_run_port 4000 --local true
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

## Experiments
- [x] ADD RBC Works
- [ ] Simulate Byzantine Nodes in all protocols
- [ ] Compare performance with 0, 1 Byzantine nodes with 4 nodes
- [ ] Compare performance with 0, 5 Byzantine nodes with 16 nodes
- [ ] Compare performance with 0, 10 Byzantine nodes with 32 nodes