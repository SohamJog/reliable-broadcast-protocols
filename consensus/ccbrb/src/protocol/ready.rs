// TODO: Call broadcast
use crate::{msg::ReadyMsg, Context, ProtMsg, Status};
use bincode;
use crypto::hash::{do_hash, Hash};
use network::{plaintcp::CancelHandler, Acknowledgement};
use reed_solomon_rs::fec::fec::FEC;
use reed_solomon_rs::fec::fec::*;
use tokio::time::{sleep, Duration};
use types::WrapperMsg;

impl Context {
    pub async fn start_ready(&mut self, c: Hash, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        if rbc_context.status != Status::READY {
            return;
        }

        let d_hashes = match rbc_context.fragments_hashes.get(&(instance_id as u64, c)) {
            Some(v) => v.clone(),
            None => {
                log::info!("No hash vector found for instance {}", instance_id);
                return;
            }
        };

        let pi_i_bytes = match bincode::serialize(&d_hashes) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::info!("Serialization failed: {}", e);
                return;
            }
        };

        let ready_msg = ReadyMsg {
            id: instance_id as u64,
            c,
            pi_i: pi_i_bytes.clone(),
            origin: self.myid,
        };

        let proto = ProtMsg::Ready(ready_msg.clone(), instance_id);
        for (replica, sec_key) in self.sec_key_map.clone() {
            if replica == self.myid {
                self.handle_ready(ready_msg.clone(), instance_id).await;
                continue;
            }

            let wrapper = WrapperMsg::new(proto.clone(), self.myid, &sec_key);
            let cancel_handler = self.net_send.send(replica, wrapper).await;
            self.add_cancel_handler(cancel_handler);
        }
    }
    pub async fn handle_ready(&mut self, msg: ReadyMsg, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        if rbc_context.status == Status::TERMINATED {
            return;
        }

        let key = (instance_id as u64, msg.c);
        let hashes = rbc_context.fragments_hashes.entry(key).or_default();
        let senders = rbc_context.ready_senders.entry(msg.c).or_default();

        if !senders.insert(msg.origin) {
            return;
        }

        let pi_i: Vec<Hash> = match bincode::deserialize(&msg.pi_i) {
            Ok(h) => h,
            Err(_) => {
                log::info!("Failed to deserialize πᵢ");
                return;
            }
        };

        // hashes.push(pi_i.clone());
        hashes.push(bincode::serialize(&pi_i).unwrap());

        if hashes.len() >= self.num_nodes - self.num_faults {
            let f = FEC::new(self.num_faults, self.num_nodes).unwrap();

            let recovered: Result<Vec<u8>, _> = f.decode(
                vec![],
                hashes
                    .iter()
                    .map(|h| {
                        let data = bincode::serialize(h).unwrap();
                        Share { number: 0, data } // number doesn't matter for decoding
                    })
                    .collect(),
            );

            match recovered {
                Ok(serialized_d) => {
                    let decoded_d: Vec<Hash> = match bincode::deserialize(&serialized_d) {
                        Ok(vec) => vec,
                        Err(_) => return,
                    };

                    let computed_c = do_hash(&serialized_d);
                    if computed_c == msg.c {
                        // We now trust D. Let's collect fragments from echo messages.
                        let mut fragments = vec![None; self.num_nodes];
                        for (h, shares) in rbc_context.received_readys.iter() {
                            if *h == msg.c {
                                for share in shares {
                                    fragments[share.number] = Some(share.clone());
                                }
                            }
                        }

                        let valid_fragments: Vec<Share> = fragments.into_iter().flatten().collect();

                        if valid_fragments.len() < self.num_nodes - self.num_faults {
                            log::info!("Not enough valid fragments for decoding");
                            return;
                        }

                        let f = FEC::new(self.num_faults, self.num_nodes).unwrap();
                        match f.decode(vec![], valid_fragments) {
                            Ok(message) => {
                                rbc_context.output_message = message.clone();
                                rbc_context.status = Status::TERMINATED;
                                log::info!("Terminated instance {}", instance_id);
                                self.terminate(message).await;
                            }
                            Err(e) => {
                                log::info!("Final decode failed: {}", e.to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    log::info!("Hash vector decoding failed: {:?}", e);
                }
            }
        }
    }
}
