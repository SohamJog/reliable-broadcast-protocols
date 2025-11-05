use consensus::reconstruct_data;

use super::init::construct_merkle_tree;
use crate::ProtMsg;
use crate::{CTRBCMsg, Context};

impl Context {
    pub async fn handle_echo(self: &mut Context, msg: CTRBCMsg, instance_id: usize) {
        /*
        1. mp verify
        2. wait until receiving n - t echos of the same root
        3. lagrange interoplate f and m
        4. reconstruct merkle tree, verify roots match.
        5. if all pass, send ready <fi, pi>
         */
        log::info!("Received echo message from node {} for RBC instance id {}", msg.origin, instance_id);
        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        if rbc_context.terminated {
            // RBC Already terminated, skip processing this message
            return;
        }
        let root = msg.mp.root();
        let echo_senders = rbc_context.echos.entry(root).or_default();

        // check if verifies
        if !echo_senders.contains_key(&msg.origin) && !msg.verify_mr_proof(&self.hash_context) {
            log::error!(
                "Invalid Merkle Proof sent by node {}, abandoning RBC instance {}",
                msg.origin,
                instance_id
            );
            return;
        }

        if !echo_senders.contains_key(&msg.origin) {
            echo_senders.insert(msg.origin, msg.shard);
        }
        
        let echo_sender = msg.origin;
        let size = echo_senders.len().clone();
        if size == self.num_nodes - self.num_faults && rbc_context.echo_root.is_none() {
            log::info!(
                "Received n-f ECHO messages for RBC Instance ID {}, sending READY message",
                instance_id
            );
            let senders = echo_senders.clone();

            // Reconstruct the entire Merkle tree
            let mut shards: Vec<Option<Vec<u8>>> = Vec::new();
            for rep in 0..self.num_nodes {
                if senders.contains_key(&rep) {
                    shards.push(Some(senders.get(&rep).unwrap().clone()));
                } else {
                    shards.push(None);
                }
            }

            let status = reconstruct_data(&mut shards, self.num_faults + 1, 2 * self.num_faults);

            if status.is_err() {
                log::error!(
                    "FATAL: Error in Lagrange interpolation {}",
                    status.err().unwrap()
                );
                return;
            }

            let shards: Vec<Vec<u8>> = shards.into_iter().map(|opt| opt.unwrap()).collect();

            let mut message = Vec::new();
            for i in 0..self.num_faults + 1 {
                message.extend(shards.get(i).clone().unwrap());
            }

            // Hashes on large messages are very expensive. Do as much as you can to avoid recomputing them.
            let my_share: Vec<u8> = shards[self.myid].clone();
            let my_proof;
            if rbc_context.fragment.is_some() && rbc_context.fragment.clone().unwrap().1.root() == root {
                my_proof = rbc_context.fragment.clone().unwrap().1;
            } else {
                my_proof = construct_merkle_tree(shards.clone(), &self.hash_context).gen_proof(self.myid);
            }
            // Reconstruct Merkle Root
            //let merkle_tree = construct_merkle_tree(shards, &self.hash_context);
            //log::info!("Reconstructing tree in echo phase for RBC instance id {}", instance_id);

            //if merkle_tree.root() == root {
                // ECHO phase is completed. Save our share and the root for later purposes and quick access.
                rbc_context.echo_root = Some(root);
                rbc_context.fragment = Some((my_share.clone(), my_proof.clone()));
                rbc_context.message = Some(message);

                // Send ready message
                let ctrbc_msg = CTRBCMsg {
                    shard: my_share,
                    mp: my_proof,
                    origin: self.myid,
                };

                self.handle_ready(ctrbc_msg.clone(), instance_id).await;
                let ready_msg = ProtMsg::Ready(ctrbc_msg, instance_id);
                self.broadcast(ready_msg).await;
            //}
        }
        // Go for optimistic termination if all n shares have appeared
        else if size == self.num_nodes {
            // log::info!(
            //     "Echo senders: {:?} for RBC instance id {}, size: {}",
            //     echo_senders,
            //     instance_id,
            //     size
            // );
            log::info!(
                "Received n ECHO messages for RBC instance id {}, terminating",
                instance_id
            );
            // Do not reconstruct the entire root again. Just send the merkle proof

            let echo_root = rbc_context.echo_root.clone();

            if echo_root.is_some() && !rbc_context.terminated {
                rbc_context.terminated = true;
                // Send Ready and terminate
                if self.crash {
                    return;
                }
                // Sending READY again is not necessary because we already sent one
                // let fragment = rbc_context.fragment.clone().unwrap();
                // let ctrbc_msg = CTRBCMsg {
                //     shard: fragment.0,
                //     mp: fragment.1,
                //     origin: self.myid,
                // };

                let message = rbc_context.message.clone().unwrap();

                // let ready_msg = ProtMsg::Ready(ctrbc_msg, instance_id);

                // self.broadcast(ready_msg).await;
                log::info!("Terminated RBC with message length {}",message.len());
                self.terminate(instance_id, message).await;
            }
        }
        log::info!("Handled echo sent by node {} for RBC instance id {}", echo_sender, instance_id);
    }
}
