use consensus::reconstruct_data;

use crate::protocol::init::construct_merkle_tree;

use crate::{CTRBCMsg, ProtMsg};

use crate::Context;
impl Context {
    // TODO: handle ready
    pub async fn handle_ready(self: &mut Context, msg: CTRBCMsg, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        log::info!("Received ready message from node {} for RBC instance id {}", msg.origin, instance_id);
        if rbc_context.terminated {
            return;
            // RBC Context already terminated, skip processing this message
        }

        let root = msg.mp.root();
        let echo_senders = rbc_context.echos.entry(root).or_default();
        let ready_senders = rbc_context.readys.entry(root).or_default();

        // Hashes on large messages are very expensive. Do as much as you can to avoid recomputing them.
        if !echo_senders.contains_key(&msg.origin) {
            if !msg.verify_mr_proof(&self.hash_context){
                log::error!(
                    "Invalid Merkle Proof sent by node {}, abandoning RBC instance {}",
                    msg.origin,
                    instance_id
                );
                return;
            }
        }
        else{
            let msg_echo = echo_senders.get(&msg.origin).unwrap().clone();
            if msg_echo != msg.shard {
                if !msg.verify_mr_proof(&self.hash_context){
                    log::error!(
                        "Invalid Merkle Proof sent by node {}, abandoning RBC instance {}",
                        msg.origin,
                        instance_id
                    );
                    return;
                }
                ready_senders.insert(msg.origin, msg.shard);
            }
            else{
                ready_senders.insert(msg.origin, msg.shard);
            }
        }
        
        let size = ready_senders.len().clone();

        if size == self.num_faults + 1 {
            // Sent ECHOs and getting a ready message for the same ECHO
            if rbc_context.echo_root.is_some() && rbc_context.echo_root.clone().unwrap() == root {
                // No need to interpolate the Merkle tree again.
                // If the echo_root variable is set, then we already sent ready for this message.
                // Nothing else to do here. Quit the execution.

                return;
            }

            let ready_senders = ready_senders.clone();

            // Reconstruct the entire Merkle tree
            log::info!("Reconstructing tree in ready phase for RBC instance id {}", instance_id);
            let mut shards: Vec<Option<Vec<u8>>> = Vec::new();
            for rep in 0..self.num_nodes {
                if ready_senders.contains_key(&rep) {
                    shards.push(Some(ready_senders.get(&rep).unwrap().clone()));
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
            let my_proof;
            if rbc_context.fragment.is_some() && rbc_context.fragment.clone().unwrap().1.root() == root {
                my_proof = rbc_context.fragment.clone().unwrap().1;
            } else {
                my_proof = construct_merkle_tree(shards.clone(), &self.hash_context).gen_proof(self.myid);
            }
            // Reconstruct Merkle Root
            //let merkle_tree = construct_merkle_tree(shards, &self.hash_context);
            //if merkle_tree.root() == root {
                // Ready phase is completed. Save our share for later purposes and quick access.
                rbc_context.fragment = Some((my_share.clone(), my_proof.clone()));

                rbc_context.message = Some(message);

                // Insert own ready share
                rbc_context
                    .readys
                    .get_mut(&root)
                    .unwrap()
                    .insert(self.myid, my_share.clone());
                // Send ready message
                let ctrbc_msg = CTRBCMsg {
                    shard: my_share,
                    mp: my_proof,
                    origin: self.myid,
                };

                let ready_msg = ProtMsg::Ready(ctrbc_msg.clone(), instance_id);

                if !self.crash {
                    self.broadcast(ready_msg).await;
                }
            //}
        } else if size == self.num_nodes - self.num_faults && !rbc_context.terminated {
            log::info!(
                "Received n-f READY messages for RBC instance id {} and message length {}, terminating",
                instance_id,
                rbc_context.message.clone().unwrap().len()
            );
            // Terminate protocol
            rbc_context.terminated = true;
            let term_msg = rbc_context.message.clone().unwrap();
            self.terminate(instance_id, term_msg).await;
        }
        log::info!("Handled ready sent by node {} for RBC instance id {}", msg.origin, instance_id);
    }
}
