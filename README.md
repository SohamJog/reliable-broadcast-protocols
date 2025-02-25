# Fault-Tolerant Computing Assignment
This assignment requires implementing two distributed protocols in Rust. 

1. Bracha's Reliable Broadcast protocol
2. Practical Byzantine Fault Tolerance (PBFT) Agreement protocol

Rust is a popular programming language which offers enhanced security guarantees because of its design of catching as many bugs as possible at compile time. Programming in Rust is an important skill, especially in understanding and implementing fault-tolerant distributed systems. For students well-versed in programming languages like Java or Python, learning to program in Rust can be challenging. However, writing code following the design principles of Rust will help students implement distributed systems with lesser exposure to attack vectors and race conditions. 

# Setting up Rust

## Hardware and OS setup
1. This artifact has been run and tested on `x86_64` and `x64` architectures. However, we are unaware of any issues that would prevent this artifact from running on `x86` architectures. 

2. This artifact has been run and tested on Ubuntu `20.04.5 LTS` OS and Raspbian Linux version released on `2023-02-21`, both of which follow the Debian distro. However, we are unaware of any issues that would prevent this artifact from running on Fedora distros like CentOS and Red Hat Linux. 

## Rust installation and Cargo setup
The repository uses the `Cargo` build tool. `Cargo` is a repository management tool used extensively in the Rust community. The compatibility between dependencies has been tested for Rust version `1.63`.


3. Run the set of following commands to install the toolchain required to compile code written in Rust and create binary executable files. 
```
$ sudo apt-get update
$ sudo apt-get -y upgrade
$ sudo apt-get -y autoremove
$ sudo apt-get -y install build-essential
$ sudo apt-get -y install cmake
$ sudo apt-get -y install curl
# Install rust (non-interactive)
$ curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
$ source $HOME/.cargo/env
$ rustup install 1.63.0
$ rustup override set 1.63.0
```
4. Build the repository using the following command. The command should be run in the directory containing the `Cargo.toml` file. 
```
$ cargo build --release
$ mkdir logs
```

5. Run the example by invoking the following command. 
```
./scripts/test.sh testdata/hyb_4/syncer Hi false
```
This process starts `n=4` processes or nodes and runs a simple Ping protocol among them. The nodes log extensively and write their logs into `{i}.log` file in the `logs` directory. Go through the logs to check if the pings are going through. Further, a process called the `syncer` manages the lifecycle of these processes. Understand the code by going through the `Ping` protocol. 

6. If the nodes hang for some reason, the ports they were using might be blocked for a long time. Use the following command to kill these processes.
```
sudo lsof -ti :7000-7015,5000 | sudo xargs kill -9
```

7. We encourage you go through the Rust lang tutorial located [here](https://doc.rust-lang.org/book/) and[here](https://rust-book.cs.brown.edu/). The first 10 chapters are sufficient for the scope of this assignment. 

# Tasks
You will be implementing two distributed protocols: a) Bracha's Reliable Broadcast (RBC) [1] protocol, and b) Practical Byzantine Fault Tolerance (PBFT) [2] Agreement protocol. In this repository, we provided the networking setup, and the necessary cryptographic tools to establish authenticated networking channels between nodes. You will have to read and understand both protocols. Next, you will have to implement these protocols in the `consensus/rbc` and `consensus/pbft` folders. There are detailed comments in the code about where to include your logic and which functions to invoke after termination. 

## Reliable Broadcast
Bracha's Reliable Broadcast protocol [1] is a fundamental primitive in asynchronous distributed systems. In this protocol, a broadcaster wishes to broadcast a string to every other node in the system. Further, the protocol must provide the guarantees of a standard RBC protocol specified in the slidedeck. A broadcasting node (In this assignment, the broadcasting node is always node `0`) will receive a value, which it will broadcast. Other nodes receive this value and terminate upon successfully completing the protocol. 

You will implement the protocol in the `consensus/rbc` folder. Please refer to the code organization section below to get a detailed idea of the codebase (This is optional). Once you terminate the protocol, invoke the `terminate` function with the received value broadcast by the broadcaster. If your protocol terminated successfully, a message like the following should be printed in the log file `syncer.log`.
```
All n nodes completed the protocol {} with value {}
```

## Practical Byzantine Fault Tolerance (PBFT)
You will also implement the PBFT [2] protocol in the steady state, i.e. when the leader is honest. As opposed to Bracha's RBC which is a broadcast protocol, PBFT is an agreement protocol where each node has an input value $v_i$. The protocol must proceed as follows.

1. Each node must send their input $v_i$ to the leader (Assume node `0` is the leader node). 
2. The leader should wait until receiving inputs from `67%` of nodes. Then, the leader should create a message with these values. Call this message $m = [v_1,v_2,...,v_{2n/3}]$. 
3. The leader should use the PBFT protocol to ensure everyone agrees on the message $m$. 
4. On achieving agreement on $m$, each node should calculate the median response in $m$ and invoke `terminate` function with it. 

The repository has already been setup in `consensus/pbft` directory. Implement the application logic in this repository. 

## Testing and Grading
For both RBC and PBFT, we will test your code over three different inputs. Once you terminate the protocol, invoke the `terminate` function with the received value broadcast by the broadcaster. An example has been provided in the `rbc` repository, which currently implements the `Ping` protocol. Note that, even in the presence of `33%` Byzantine faulty nodes, the remaining honest nodes must still successfully terminate the protocol. We will test your code by starting `33%` nodes in the Byzantine mode, after which it should still terminate. For PBFT, we ensure that the leader node is not faulty in this case. 

The grading will be performed as follows.
0. 20 pts - Learning the basics of Rust.
1. 20 pts - Bracha's RBC protocol must successfully terminate at all nodes when all nodes are honest.  
2. 20 pts - Bracha's RBC protocol must successfully terminate at all honest nodes when `33%` nodes are Byzantine faulty.
3. 20 pts - PBFT protocol must successfully terminate at all nodes when all nodes are honest. 
4. 20 pts - PBFT protocol must successfully terminate at all honest nodes when `33%` nodes are Byzantine faulty.

We estimate an effort of two hours to learn Rust, 4 hours to learn and implement Bracha's RBC, and 4 hours to implement PBFT. 

### Extra credit assignment
Mind that, in PBFT, if the leader node is a Byzantine fault, no node outputs a value. Implement a *View-Change* protocol [2] to receive extra credit. Note that a View Change would require a timer library, which keeps track of an unresponsive leader. Nodes then oust the unresponsive leader and install a new leader in its place. Refer to the PBFT view change protocol and implement it to receive extra credit of 50 points.

# Code Organization
The artifact is organized into the following modules of code. 
1. The `consensus` directory contains the implementations of various protocols. Each protocol contains a `context.rs` file, which contains a function named `spawn` from where the protocol's execution starts. This function is called by the `node` library in the `node` folder. This library contains a `main.rs` file, which spawns an instance of a node running the respective protocol by invoking the `spawn` function. 

2. The `crypto` directory contains code that manages the pairwise authenticated channels between nodes. Mainly, nodes use Message Authentication Codes (MACs) for message authentication. This repo manages the required secret keys and mechanisms for generating MACs. You do not need to make changes to this directory. 

3. The `types` directory governs the message serialization and deserialization. Each message sent between nodes is serialized into bytecode to be sent over the network. Upon receiving a message, each node deserializes the received bytecode into the required message type after receiving. Modify the `ProtMsg` enum in this library to define custom message types. You do not need to worry about serializing and deserializing these messages. The library automatically takes care of this for you. 

4. *Networking*: This repository uses the `libnet-rs` (https://github.com/libdist-rs/libnet-rs) networking library. Similar libraries include networking library from the `narwhal` (https://github.com/MystenLabs/sui/tree/main/narwhal/) repository. The nodes use the `tcp` protocol to send messages to each other. You do not need to make any changes to this directory. 

5. The `tools` directory consists of code that generates configuration files for nodes. You do not need to make any changes to this directory. 

6. The `config` directory contains code pertaining to configuring each node in the distributed system. Each node requires information about port to use, network addresses of other nodes, symmetric keys to establish pairwise authenticated channels between nodes, and protocol specific configuration parameters. Code related to managing and parsing these parameters is in the `config` directory. You do not need to make any changes to this directory. 

## Relevance to applications in the industry 
Bracha's RBC and other RBC protocols are being used in current operational permissioned blockchains like `Aptos` and `Sui`, both of which handle transactions worth millions of dollars each day. 

The described version of PBFT is used in `Chainlink` as an Oracle agreement protocol, which ensures that the nodes agree on a value within the `Convex-Hull` of their inputs. `Chainlink` reports an estimated 300 million requests per year on their Oracle service. 

# References
[1] Living with Asynchrony: Bracha's Reliable Broadcast: https://decentralizedthoughts.github.io/2020-09-19-living-with-asynchrony-brachas-reliable-broadcast/

[2] Castro, Miguel, and Barbara Liskov. "Practical byzantine fault tolerance." In OsDI, vol. 99, no. 1999, pp. 173-186. 1999.# ftc-course-assignment


# Instructions to add another protocol
1. Add a directory `protocol-name` in the `consensus` directory. Pattern-match with `rbc` or `pbft`

2. In `node/Cargo.toml`, add the line `protocol-name = {package = "protocol-name",path="../consensus/protocol-name"}`

3. In `node/main.rs`, add the new protocol in the `match vss_type`, pattern-match with other protocols. 

4. Before running, change line 24 in `scripts/test.sh` to `--protocol protocol-name` 

# Additional Notes
## Script to test with test_msgs.txt
```bash
./scripts/test.sh testdata/hyb_16/syncer Hi false testdata/test_msgs.txt
```

## Script to test with n number of nodes (in case you lose config by switching branches)
```bash
cd testdata
mkdir hyb_16
```
```bash
./target/release/genconfig --NumNodes 16 --delay 10 --blocksize 100 --client_base_port 7000 --target testdata/hyb_16/ --payload 100 --out_type json --base_port 9000 --client_run_port 4000 --local true
```
