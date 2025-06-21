use crate::{
    msg::{ProtMsg, ReadyMsg},
    Context, Status,
};
use bincode;
use consensus::reconstruct_data;
use crypto::hash::{do_hash, Hash};
use network::{plaintcp::CancelHandler, Acknowledgement};
use reed_solomon_rs::fec::fec::{Share, FEC};
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

        hashes.push(bincode::serialize(&pi_i).unwrap());

        if hashes.len() >= self.num_nodes - self.num_faults {
            let f = FEC::new(self.num_faults, self.num_nodes).unwrap();

            // ECCDec(t + 1, e, hashes)
            let recovered = f
                .decode(
                    vec![],
                    hashes
                        .iter()
                        .enumerate()
                        .map(|(i, h)| Share {
                            number: i,
                            data: h.clone(),
                        })
                        .collect(),
                )
                .map_err(|e| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    )) as Box<dyn std::error::Error + Send + Sync>
                });

            match recovered {
                Ok(serialized_d) => {
                    let decoded_d: Vec<Hash> = match bincode::deserialize(&serialized_d) {
                        Ok(vec) => vec,
                        Err(_) => {
                            log::info!("Failed to deserialize D'");
                            return;
                        }
                    };

                    let computed_c = do_hash(&serialized_d);
                    if computed_c != msg.c {
                        log::info!("c mismatch: H(D') != c");
                        return;
                    }

                    // Wait for t+1 fragments whose hash ∈ D'
                    let mut fragment_options = vec![None; self.num_nodes];
                    for (h, shares) in rbc_context.received_readys.iter() {
                        if *h == msg.c {
                            for share in shares {
                                if decoded_d.get(share.number) == Some(&do_hash(&share.data)) {
                                    fragment_options[share.number] = Some(share.data.clone());
                                }
                            }
                        }
                    }

                    let mut fragments = fragment_options.clone();
                    let threshold = self.num_faults + 1;
                    let available = fragments.iter().filter(|s| s.is_some()).count();

                    if available < threshold {
                        log::info!("Not enough valid fragments to reconstruct M");
                        return;
                    }

                    // Reconstruct M (ECDec)
                    match reconstruct_data(
                        &mut fragments,
                        self.num_faults + 1,
                        self.num_nodes - self.num_faults - 1,
                    ) {
                        Ok(_) => {
                            let message = fragments
                                .into_iter()
                                .flatten()
                                .flatten()
                                .collect::<Vec<u8>>();

                            // re-encode to verify against D'
                            let f = FEC::new(self.num_faults, self.num_nodes).unwrap();
                            let mut hashes_match = true;

                            for (i, block) in message.chunks(message.len() / threshold).enumerate()
                            {
                                let hash = do_hash(block);
                                if decoded_d.get(i) != Some(&hash) {
                                    hashes_match = false;
                                    break;
                                }
                            }

                            if hashes_match {
                                rbc_context.output_message = message.clone();
                                rbc_context.status = Status::TERMINATED;
                                log::info!("Terminated instance {} with output", instance_id);
                                self.terminate(message).await;
                            } else {
                                log::info!("Hash mismatch on reconstructed message: ⊥");
                                rbc_context.status = Status::TERMINATED;
                                let _ = rbc_context;
                                self.terminate(vec![]).await;
                            }
                        }
                        Err(e) => {
                            log::info!("Reconstruction failed: {}", e);
                            rbc_context.status = Status::TERMINATED;
                            let _ = rbc_context;
                            self.terminate(vec![]).await; // ⊥
                        }
                    }
                }
                Err(e) => {
                    log::info!("Error decoding D': {:?}", e);
                }
            }
        }
    }
}
