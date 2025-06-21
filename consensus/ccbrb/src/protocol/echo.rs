use crate::msg::{EchoMsg, SendMsg};
use crate::Status;
use crate::{Context, ProtMsg};
use bincode;
use crypto::hash::{do_hash, Hash};
use network::{plaintcp::CancelHandler, Acknowledgement};
use reed_solomon_rs::fec::fec::*;
use types::WrapperMsg;

impl Context {
    pub async fn start_echo(&mut self, msg: SendMsg, instance_id: usize) {
        let d_hashes = msg.d_hashes.clone(); // D = [H(d₁), ..., H(dₙ)]
        let c = do_hash(&bincode::serialize(&d_hashes).unwrap()); // c = H(D)

        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        rbc_context.fragment = msg.d_j.clone();
        rbc_context.status = Status::ECHO;

        for replica in 0..self.num_nodes {
            let share = if self.byz && replica != self.myid {
                Share {
                    number: replica,
                    data: vec![],
                }
            } else {
                msg.d_j.clone()
            };

            let echo_msg = EchoMsg {
                id: instance_id as u64,
                d_i: share,
                pi_i: bincode::serialize(&d_hashes).unwrap(), // πᵢ
                c,
                origin: self.myid,
            };

            let proto_msg = ProtMsg::Echo(echo_msg.clone(), instance_id);
            if replica == self.myid {
                self.handle_echo(echo_msg.clone(), instance_id).await;
                continue;
            }

            let sec_key = &self.sec_key_map[&replica];
            let wrapped = WrapperMsg::new(proto_msg.clone(), self.myid, sec_key);
            let cancel_handler: CancelHandler<Acknowledgement> =
                self.net_send.send(replica, wrapped).await;
            self.add_cancel_handler(cancel_handler);
        }
    }

    pub async fn handle_echo(&mut self, echo_msg: EchoMsg, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        let senders = rbc_context.echo_senders.entry(echo_msg.c).or_default();
        if !senders.insert(echo_msg.origin) {
            return; // duplicate
        }

        *rbc_context
            .received_echo_count
            .entry(echo_msg.c)
            .or_default() += 1;

        let (max_count, mode_content) = rbc_context.get_max_echo_count();
        if max_count >= self.num_nodes - self.num_faults && rbc_context.status == Status::ECHO {
            if let Some(hash) = mode_content {
                rbc_context.status = Status::READY;
                self.start_ready(hash, instance_id).await;
            }
        }
    }
}
