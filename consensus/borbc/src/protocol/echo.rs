
use consensus::reconstruct_data;

use super::init::construct_merkle_tree;
use crate::ProtMsg;
use crate::{CTRBCMsg, Context};

impl Context {
    pub async fn handle_echo(self: &mut Context, msg: CTRBCMsg, instance_id: usize) {
        let root = msg.mp.root();
        
        // Use an inner scope to limit the lifetime of the mutable borrow 'rbc_context'
        let (
            should_broadcast_vote, 
            vote_msg, 
            should_broadcast_ready_1, 
            ready_msg_1, 
            should_reconstruct_opt_commit, 
            should_reconstruct_latch, 
            ready_quorum_reached,
            should_reconstruct_nf,
            should_terminate_n,
            ready_msg_n,
            message_n
        ) = {
            let rbc_context = self.rbc_context.entry(instance_id).or_default();

            if rbc_context.terminated {
                // RBC Already terminated, skip processing this message
                return;
            }
            
            // Check if verifies
            if !msg.verify_mr_proof(&self.hash_context) {
                log::error!(
                    "Invalid Merkle Proof sent by node {}, abandoning RBC instance {}",
                    msg.origin,
                    instance_id
                );
                return;
            }

            let echo_senders = rbc_context.echos.entry(root).or_default();

            if echo_senders.contains_key(&msg.origin) {
                return;
            }

            echo_senders.insert(msg.origin, msg.shard.clone());

            let size = echo_senders.len(); // '.clone()' is unnecessary for len()

            let mut should_broadcast_vote = false;
            let mut vote_msg_opt = None;
            let mut should_broadcast_ready_1 = false;
            let mut ready_msg_1_opt = None;
            let mut should_reconstruct_opt_commit = false;
            let mut should_reconstruct_latch = false;
            let mut should_reconstruct_nf = false;
            let mut should_terminate_n = false;
            let mut ready_msg_n_opt = None;
            let mut message_n_opt = None;

            // 1) Send Vote at ceil(n/2)
            let vote_thresh = (self.num_nodes + 1) / 2;
            if !rbc_context.sent_vote && size >= vote_thresh {
                rbc_context.sent_vote = true;
                
                // Extract necessary data before borrowing 'self' for broadcast
                let (my_shard, my_mp) = if let Some((s, p)) = rbc_context.fragment.clone() {
                    (s, p)
                } else {
                    (msg.shard.clone(), msg.mp.clone())
                }; 
                
                should_broadcast_vote = true;
                vote_msg_opt = Some(CTRBCMsg { shard: my_shard, mp: my_mp, origin: self.myid });
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
                should_broadcast_ready_1 = true;
                ready_msg_1_opt = Some(CTRBCMsg { shard: my_shard, mp: my_mp, origin: self.myid });
            }

            let opt_commit_thresh = (self.num_nodes + 2*self.num_faults - 2 + 1) / 2;
            if !rbc_context.terminated && size >= opt_commit_thresh {
                should_reconstruct_opt_commit = true;
            }

            let latch_echo_thresh = (self.num_nodes - self.num_faults + 1 + 1) / 2;
            if rbc_context.ready_quorum_reached && !rbc_context.terminated && size >= latch_echo_thresh {
                should_reconstruct_latch = true;
            }

            if size == self.num_nodes - self.num_faults {
                should_reconstruct_nf = true;
            }
            
            if size == self.num_nodes {
                 let echo_root = rbc_context.echo_root.clone();
                 if echo_root.is_some() && !rbc_context.terminated {
                    // Extract data for termination
                    let fragment = rbc_context.fragment.clone().unwrap();
                    let message = rbc_context.message.clone().unwrap();
                    
                    should_terminate_n = true;
                    rbc_context.terminated = true;
                    ready_msg_n_opt = Some(CTRBCMsg {
                        shard: fragment.0,
                        mp: fragment.1,
                        origin: self.myid,
                    });
                    message_n_opt = Some(message);
                }
            }

            let echo_senders_clone = if should_reconstruct_opt_commit || should_reconstruct_latch || should_reconstruct_nf {
                Some(echo_senders.clone())
            } else {
                None
            };
            
            (
                should_broadcast_vote, 
                vote_msg_opt, 
                should_broadcast_ready_1, 
                ready_msg_1_opt, 
                should_reconstruct_opt_commit, 
                should_reconstruct_latch, 
                rbc_context.ready_quorum_reached,
                should_reconstruct_nf,
                should_terminate_n,
                ready_msg_n_opt,
                message_n_opt
            )
        }; 
        
        if !self.crash {
            if should_broadcast_vote {
                self.broadcast(ProtMsg::Vote(vote_msg.unwrap(), instance_id)).await;
            }
            if should_broadcast_ready_1 {
                self.broadcast(ProtMsg::Ready(ready_msg_1.unwrap(), instance_id)).await;
            }
        }
        
        if should_reconstruct_opt_commit || (ready_quorum_reached && should_reconstruct_latch) || should_reconstruct_nf {
            let senders = self.rbc_context.get(&instance_id).unwrap().echos.get(&root).unwrap().clone();
            
            let mut shards_opt: Vec<Option<Vec<u8>>> = (0..self.num_nodes).map(|rep|
                 senders.get(&rep).cloned()
             ).collect();

            
            let status = reconstruct_data(&mut shards_opt, self.num_faults + 1, 2 * self.num_faults); // Added +1 to 2*self.num_faults to match Reed-Solomon 'n'
            
            if let Err(e) = status {
                log::error!("FATAL: Error in Lagrange interpolation {}", e);
                return;
            }
            let shards: Vec<Vec<u8>> = shards_opt.into_iter().map(|opt| opt.unwrap()).collect(); 

            let mut message = Vec::new();
            for i in 0..self.num_faults + 1 { 
                message.extend(shards.get(i).expect("Missing share after reconstruction")); 
            }
            let my_share: Vec<u8> = shards[self.myid].clone();

            let merkle_tree = construct_merkle_tree(shards.clone(), &self.hash_context);
            
            if merkle_tree.root() == root {
                let my_mp = merkle_tree.gen_proof(self.myid);
                let out_msg = CTRBCMsg { shard: my_share.clone(), mp: my_mp.clone(), origin: self.myid };

                let mut rbc_context = self.rbc_context.get_mut(&instance_id).unwrap();
                
                rbc_context.echo_root = Some(root);
                rbc_context.fragment = Some((my_share.clone(), my_mp.clone()));
                rbc_context.message = Some(message.clone());
                
                if should_reconstruct_opt_commit || should_reconstruct_latch {
                     rbc_context.terminated = true;
                     
                     if !self.crash {
                         self.broadcast(ProtMsg::Ready(out_msg.clone(), instance_id)).await;
                     }
                     self.terminate(message).await;
                     return;
                }
                
                if should_reconstruct_nf {
                    log::info!(
                        "Received n-f ECHO messages for RBC Instance ID {}, sending READY message",
                        instance_id
                    );
                    
                    self.handle_ready(out_msg.clone(), instance_id).await;
                    let ready_msg = ProtMsg::Ready(out_msg, instance_id);
                    self.broadcast(ready_msg).await;
                }
            }
        }
        
        // Handle Optimistic termination at n
        if should_terminate_n {
            log::info!(
                "Received n ECHO messages for RBC instance id {}, terminating",
                instance_id
            );
            
            if !self.crash {
                let ready_msg = ProtMsg::Ready(ready_msg_n.unwrap(), instance_id);
                self.broadcast(ready_msg).await;
            }
            self.terminate(message_n.unwrap()).await;
        }
    }
}
