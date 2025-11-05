use crate::{CTRBCMsg, ProtMsg};
use crate::Context;

impl Context {
    pub async fn handle_vote(self: &mut Context, msg: CTRBCMsg, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        if rbc_context.terminated { 
            return; 
        }

        let root = msg.mp.root();
        // Record this vote
        let echo_senders = rbc_context.echos.entry(root).or_default();
        let vote_senders = rbc_context.votes.entry(root).or_default();
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
                vote_senders.insert(msg.origin, msg.shard.clone());
            }
            else{
                vote_senders.insert(msg.origin, msg.shard.clone());
            }
        }
        
        let votes_now = vote_senders.len();

        // If we haven't sent READY yet and we have enough VOTEs (ceil((n+f-1)/2)), send READY
        let vote_ready_thresh = (self.num_nodes + self.num_faults - 1 + 1) / 2; // ceil((n+f-1)/2)
        if !rbc_context.sent_ready && votes_now >= vote_ready_thresh {
            rbc_context.sent_ready = true;

            // If we already have our fragment (set in Echo), reuse it; otherwise derive proof from this vote's root.
            let (my_shard, my_mp) = if let Some((shard, proof)) = rbc_context.fragment.clone() {
                (shard, proof)
            } else {
                // we may not have reconstructed yet; send a fresh proof for our own id
                // NOTE: This requires we have the Merkle tree to gen_proof; if we don't, just forward msg's root with our shard if known.
                // Minimal change: reuse msg.mp to carry the root and our shard if we have one.
                (msg.shard.clone(), msg.mp.clone())
            };

            let out = CTRBCMsg { shard: my_shard, mp: my_mp, origin: self.myid };
            if !self.crash {
                self.broadcast(ProtMsg::Ready(out, instance_id)).await;
            }
        }
    }
}
