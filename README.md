# Reliable Broadcast Protocols

This repository contains a Rust implementation of several asynchronous reliable broadcast (RBC) protocols, including:

- **Asynchronous Data Dissemination (ADD-RBC)** — based on [Das et al. (2021)](https://eprint.iacr.org/2021/777.pdf)
- **Asynchronous Verifiable Information Dispersal (CTRBC)** — based on [Cachin and Tessaro (2005)](https://homes.cs.washington.edu/~tessaro/papers/dds.pdf)
- **Cross-Checksum Reliable Broadcast (CCRBC)** — based on [Alhaddad et al. (2022)](https://eprint.iacr.org/2022/776.pdf)
- **Bracha’s Classic RBC** — baseline protocol from [Bracha (1987)]

---

## Directory Structure

This repository organizes protocol implementations under the `consensus` directory:

- `consensus/rbc`  
  Contains Bracha’s original RBC protocol, which incurs $\mathcal{O}(n^2 |M|)$ communication cost due to full-message retransmission by every node.

- `consensus/ctrbc`  
  Contains the Cachin-Tessaro RBC protocol (CTRBC), which achieves $\mathcal{O}(n |M| + \kappa n^2 \log n)$ communication complexity by dispersing erasure-coded fragments with Merkle tree commitments. We implement optimistic termination, allowing it to complete in 2 rounds under honest behavior. This design trades lower bandwidth for higher per-node computation due to Lagrange interpolation.

- `consensus/addrbc`  
  Implements ADD-RBC from Das et al., achieving $\mathcal{O}(n |M| + \kappa n^2)$ communication. It relies on online error correction (OEC) applied to the full message, which introduces significant computational overhead.

- `consensus/ccbrb`  
  Contains CCRBC, based on Alhaddad et al.’s variant of ADD-RBC. It achieves the same communication complexity as ADD-RBC but applies OEC only to small fixed-size digests, reducing computational cost without affecting bandwidth.

---

## Purpose

We implement and benchmark these protocols to empirically evaluate trade-offs in:

- **Round complexity**
- **Communication complexity**
- **Per-node computational cost**

This benchmark-driven study helps illuminate practical performance characteristics of RBC protocols under various fault and deployment settings. Since our benchmarks are run in an n-parallel setting where each node acts as a sender, the imbalance in per-node computation costs across protocols is naturally averaged out.


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
