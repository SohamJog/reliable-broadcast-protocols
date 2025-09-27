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
        
        let shard = msg.shard.clone();
        let proof = msg.mp.clone();


        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        if rbc_context.terminated {
            // RBC Already terminated, skip processing this message
            return;
        }
        // check if verifies
        if !msg.verify_mr_proof(&self.hash_context) {
            log::error!(
                "Invalid Merkle Proof sent by node {}, abandoning RBC instance {}",
                msg.origin,
                instance_id
            );
            return;
        }

        let root = msg.mp.root();
        let echo_senders = rbc_context.echos.entry(root).or_default();

        if echo_senders.contains_key(&msg.origin) {
            return;
        }

        echo_senders.insert(msg.origin, shard.clone());

        let size = echo_senders.len().clone();

        // changes for borbc
        //. Upon receiving ⟨echo, 𝑠∗, 𝑤∗, ℎ⟩ from ⌈
        // 𝑛
        // 2
        // ⌉ non-broadcaster parties with
        // valid Merkle proofs, let 𝑀 = verify_interpolation({𝑠∗ }, h). If 𝑀 ≠ ⊥, send
        // ⟨vote, 𝑠𝑗
        // , 𝑤𝑗
        // , ℎ⟩ to party 𝑃𝑗 ∀𝑗 ∈ [𝑛] if not already sent.

        // 1) Send Vote at ceil(n/2)
        let vote_thresh = (self.num_nodes + 1) / 2;
        if !rbc_context.sent_vote && size >= vote_thresh {
            rbc_context.sent_vote = true;

            // Use our fragment if we already have it (set when we reconstruct), else forward the root via this proof
            let (my_shard, my_mp) = if let Some((s, p)) = rbc_context.fragment.clone() {
                (s, p)
            } else {
                (shard.clone(), proof.clone())
            }; 

            let vote_msg = CTRBCMsg { shard: my_shard, mp: my_mp, origin: self.myid };
            if !self.crash {
                self.broadcast(ProtMsg::Vote(vote_msg, instance_id)).await;
            }
        }

        // 2) Send Ready at ceil((n+f-1)/2)
        let ready_by_echo_thresh = (self.num_nodes + self.num_faults - 1 + 1) / 2;
        
        if !rbc_context.sent_ready && size >= ready_by_echo_thresh {
            rbc_context.sent_ready = true;
            let (my_shard, my_mp) = if let Some((shard, proof)) = rbc_context.fragment.clone() {
                (shard, proof)
            } else {
                (msg.shard.clone(), msg.mp.clone())
            };
            let ready_msg = CTRBCMsg { shard: my_shard, mp: my_mp, origin: self.myid };
            if !self.crash {
                self.broadcast(ProtMsg::Ready(ready_msg, instance_id)).await;
            }
        }

        // 3) Opt Commit at ceil((n+2f-2)/2)  => decode & terminate early
        let opt_commit_thresh = (self.num_nodes + 2*self.num_faults - 2 + 1) / 2;
        if !rbc_context.terminated && size >= opt_commit_thresh {
            // Reconstruct (exactly like your existing n-f branch):
            let senders = echo_senders.clone();
            let mut shards: Vec<Option<Vec<u8>>> = (0..self.num_nodes).map(|rep|
                senders.get(&rep).cloned()
            ).collect();

            let status = reconstruct_data(&mut shards, self.num_faults + 1, 2 * self.num_faults);
            if let Err(e) = status {
                log::error!("FATAL: Error in Lagrange interpolation {}", e);
                return;
            }
            let shards: Vec<Vec<u8>> = shards.into_iter().map(|opt| opt.unwrap()).collect();

            let mut message = Vec::new();
            for i in 0..self.num_faults + 1 { message.extend(shards.get(i).unwrap()); }
            let my_share: Vec<u8> = shards[self.myid].clone();

            // Recompute Merkle root and verify
            let merkle_tree = construct_merkle_tree(shards, &self.hash_context);
            if merkle_tree.root() == root {
                rbc_context.echo_root = Some(root);
                rbc_context.fragment = Some((my_share.clone(), merkle_tree.gen_proof(self.myid)));
                rbc_context.message = Some(message);
                rbc_context.terminated = true;

                // Optional: broadcast Ready (harmless), then terminate
                if !self.crash {
                    let out = CTRBCMsg { shard: my_share, mp: merkle_tree.gen_proof(self.myid), origin: self.myid };
                    self.broadcast(ProtMsg::Ready(out, instance_id)).await;
                }
                let final_msg = rbc_context.message.clone().unwrap();
                self.terminate(final_msg).await;
                return;
            }
        }

        let latch_echo_thresh = (self.num_nodes - self.num_faults + 1 + 1) / 2;
        if rbc_context.ready_quorum_reached && !rbc_context.terminated && size >= latch_echo_thresh {

            let senders = echo_senders.clone();
            let mut shards: Vec<Option<Vec<u8>>> = (0..self.num_nodes).map(|rep|
                senders.get(&rep).cloned()
            ).collect();

            let status = reconstruct_data(&mut shards, self.num_faults + 1, 2 * self.num_faults);
            if let Err(e) = status {
                log::error!("FATAL: Error in Lagrange interpolation {}", e);
                return;
            }
            let shards: Vec<Vec<u8>> = shards.into_iter().map(|opt| opt.unwrap()).collect();

            let mut message = Vec::new();
            for i in 0..self.num_faults + 1 { message.extend(shards.get(i).unwrap()); }
            let my_share: Vec<u8> = shards[self.myid].clone();

            let merkle_tree = construct_merkle_tree(shards, &self.hash_context);
            if merkle_tree.root() == root {
                rbc_context.echo_root = Some(root);
                rbc_context.fragment = Some((my_share.clone(), merkle_tree.gen_proof(self.myid)));
                rbc_context.message = Some(message);
                rbc_context.terminated = true;

                if !self.crash {
                    let out = CTRBCMsg { shard: my_share, mp: merkle_tree.gen_proof(self.myid), origin: self.myid };
                    self.broadcast(ProtMsg::Ready(out, instance_id)).await;
                }
                let final_msg = rbc_context.message.clone().unwrap();
                self.terminate(final_msg).await;
                return;
            }
        }



        // end changes for borbc


        if size == self.num_nodes - self.num_faults {
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

            let my_share: Vec<u8> = shards[self.myid].clone();

            // Reconstruct Merkle Root
            let merkle_tree = construct_merkle_tree(shards, &self.hash_context);
            if merkle_tree.root() == root {
                // ECHO phase is completed. Save our share and the root for later purposes and quick access.
                rbc_context.echo_root = Some(root);
                rbc_context.fragment = Some((my_share.clone(), merkle_tree.gen_proof(self.myid)));
                rbc_context.message = Some(message);

                // Send ready message
                let ctrbc_msg = CTRBCMsg {
                    shard: my_share,
                    mp: merkle_tree.gen_proof(self.myid),
                    origin: self.myid,
                };

                self.handle_ready(ctrbc_msg.clone(), instance_id).await;
                let ready_msg = ProtMsg::Ready(ctrbc_msg, instance_id);
                self.broadcast(ready_msg).await;
            }
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

                let fragment = rbc_context.fragment.clone().unwrap();
                let ctrbc_msg = CTRBCMsg {
                    shard: fragment.0,
                    mp: fragment.1,
                    origin: self.myid,
                };

                let message = rbc_context.message.clone().unwrap();

                let ready_msg = ProtMsg::Ready(ctrbc_msg, instance_id);

                self.broadcast(ready_msg).await;
                self.terminate(message).await;
            }
        }
    }
}
