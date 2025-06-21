use crate::{
    msg::{ProtMsg, ReadyMsg},
    Context, Status,
};
use bincode;
use consensus::{get_shards, reconstruct_data};
use crypto::hash::{do_hash, Hash};
use network::{plaintcp::CancelHandler, Acknowledgement};
use reed_solomon_rs::fec::fec::{Share, FEC};
use std::collections::HashSet;
use types::WrapperMsg;

impl Context {
    pub async fn start_ready(&mut self, c: Hash, pi_i: Share, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        if rbc_context.status != Status::READY {
            return;
        }

        // let d_hashes = match rbc_context.fragments_hashes.get(&(instance_id as u64, c)) {
        //     Some(v) => v.clone(),
        //     None => {
        //         log::info!("No hash vector found for instance {}", instance_id);
        //         return;
        //     }
        // };

        // rbc_context
        //     .fragments_hashes
        //     .insert((instance_id as u64, c), d_hashes.clone());

        // let pi_i_bytes = match bincode::serialize(&d_hashes) {
        //     Ok(bytes) => bytes,
        //     Err(e) => {
        //         log::info!("Serialization failed: {}", e);
        //         return;
        //     }
        // };

        let ready_msg = ReadyMsg {
            id: instance_id as u64,
            c,
            pi_i: pi_i.clone(),
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
        let mut cancel_handlers = vec![];

        {
            let rbc_context = self.rbc_context.entry(instance_id).or_default();
            if rbc_context.status == Status::TERMINATED {
                return;
            }

            // pub struct ReadyMsg {
            //     pub id: u64,
            //     pub c: Hash,
            //     pub pi_i: Share,
            //     pub origin: Replica,
            // }
            let pi_i = msg.pi_i.clone();

            let pi_i_serialized = bincode::serialize(&pi_i).unwrap();
            // Track senders per (c, πᵢ)
            let pi_i_map = rbc_context.ready_senders.entry(msg.c).or_default();
            let senders = pi_i_map.entry(pi_i_serialized.clone()).or_default();

            if !senders.insert(msg.origin) {
                return; // duplicate
            }

            let hashes_entry = rbc_context
                .fragments_hashes
                .entry((instance_id as u64, msg.c))
                .or_default();
            hashes_entry.push(pi_i.clone());

            //if (not yet sent ⟨𝑖𝑑, READY, 𝑐⟩ and received 𝑡 + 1 ⟨READY⟩ messages with the same 𝑐) then

            if !rbc_context.sent_ready {
                let threshold = self.num_faults + 1;
                for (pi_i_bytes, ready_senders) in pi_i_map.iter() {
                    //  wait for 𝑡 + 1 ⟨ECHO⟩ messages with the same 𝑐 and 𝜋�
                    if ready_senders.len() >= threshold {
                        if let Some(echo_map) = rbc_context.echo_senders.get(&msg.c) {
                            if let Some(echo_senders) = echo_map.get(pi_i_bytes) {
                                if echo_senders.len() >= threshold {
                                    rbc_context.sent_ready = true;
                                    // let pi_i: Share = bincode::deserialize(pi_i_bytes).unwrap();
                                    let pi_i: Share = bincode::deserialize(pi_i_bytes).unwrap();
                                    let pi_i_cloned = pi_i.clone();

                                    let ready_msg = ReadyMsg {
                                        id: instance_id as u64,
                                        c: msg.c,
                                        pi_i: pi_i.clone(),
                                        origin: self.myid,
                                    };

                                    let proto = ProtMsg::Ready(ready_msg.clone(), instance_id);

                                    for (replica, sec_key) in self.sec_key_map.clone() {
                                        if replica == self.myid {
                                            // Directly mutate context without awaiting recursion
                                            // let pi_i_serialized = bincode::serialize(&pi_i).unwrap();
                                            let pi_i_serialized =
                                                bincode::serialize(&pi_i_cloned.clone()).unwrap();

                                            let local_ready_map =
                                                rbc_context.ready_senders.entry(msg.c).or_default();
                                            let local_senders =
                                                local_ready_map.entry(pi_i_serialized).or_default();
                                            local_senders.insert(self.myid);

                                            let hashes_entry = rbc_context
                                                .fragments_hashes
                                                .entry((instance_id as u64, msg.c))
                                                .or_default();
                                            hashes_entry.push(pi_i_cloned.clone());
                                            continue;
                                        }

                                        let wrapper =
                                            WrapperMsg::new(proto.clone(), self.myid, &sec_key);
                                        let cancel_handler =
                                            self.net_send.send(replica, wrapper).await;
                                        cancel_handlers.push(cancel_handler);
                                    }

                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        // drop(&mut *rbc_context);

        for handler in cancel_handlers {
            self.add_cancel_handler(handler);
        }

        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        // Online error correction
        let hash_shares = rbc_context
            .fragments_hashes
            .get(&(instance_id as u64, msg.c))
            .cloned()
            .unwrap_or_default();

        // if 𝑓 𝑟𝑎𝑔𝑚𝑒𝑛𝑡𝑠ℎ𝑎𝑠ℎ𝑒𝑠 [(𝑖𝑑, 𝑐)] ≥ 2𝑡 + 1 then
        if hash_shares.len() >= 2 * self.num_faults + 1 {
            // check if the length of data of all shares is consistent
            let data_length = hash_shares[0].data.len();
            if !hash_shares
                .iter()
                .all(|share| share.data.len() == data_length)
            {
                log::warn!("Inconsistent data lengths in hash shares, cannot proceed");
                return;
            }

            let mut f = FEC::new(self.num_faults, self.num_nodes).unwrap();

            let d_prime = match f.decode(vec![], hash_shares.clone()) {
                Ok(data) => data,
                Err(_) => {
                    log::warn!("Could not reconstruct D′ from hash shares, trying higher error tolerance later");
                    return; // e → e + 1 logic happens in later retries
                }
            };

            // if 𝐻(𝐷′) = 𝑐 then
            if do_hash(&d_prime) == msg.c {
                // log::info!("show D′: {:?} instance id: {}", d_prime, instance_id);
                let valid_hashes: HashSet<Hash> = d_prime
                    .chunks(32) // assuming each hash is 32 bytes
                    // .filter(|chunk| chunk.len() == 32) // ensure correct length
                    .map(|chunk| {
                        // log::info!("chunk: {:?}. instance_id: {}", chunk, instance_id);
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(chunk);
                        arr
                    })
                    .collect();

                // log::info!(
                //     "D′ reconstructed successfully with {} valid hashes",
                //     valid_hashes.len()
                // );

                let data_shares = rbc_context
                    .fragments_data
                    .get(&(instance_id as u64, msg.c))
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|share| valid_hashes.contains(&do_hash(&share.data)))
                    .collect::<Vec<_>>();

                // wait for t+1 ⟨ECHO⟩ message where 𝐻(𝑑𝑗) ∈ 𝐷′and filter 𝑓𝑟𝑎𝑔𝑚𝑒𝑛𝑡𝑠𝑑𝑎𝑡𝑎[(𝑖𝑑, 𝑐)] accordingly
                if data_shares.len() < self.num_faults + 1 {
                    return; // wait for more
                }

                let mut input_shares: Vec<Option<Vec<u8>>> = vec![None; self.num_nodes];

                for share in &data_shares {
                    input_shares[share.number] = Some(share.data.clone());
                }

                if reconstruct_data(
                    &mut input_shares,
                    self.num_nodes - self.num_faults,
                    self.num_faults,
                )
                .is_err()
                {
                    log::warn!("reconstruct_data failed");
                    return;
                }

                let mut reconstructed_data = vec![];
                for maybe in input_shares.iter().take(self.num_nodes - self.num_faults) {
                    if let Some(ref block) = maybe {
                        reconstructed_data.extend_from_slice(block);
                    }
                }

                // re encoding d′ = ECEnc(M)

                let recomputed_shards = get_shards(
                    reconstructed_data.clone(),
                    self.num_nodes - self.num_faults,
                    self.num_faults,
                );
                let d_prime_hashes: Vec<Hash> = d_prime
                    .chunks(32)
                    .map(|chunk| {
                        let mut h = [0u8; 32];
                        h.copy_from_slice(chunk);
                        h
                    })
                    .collect();

                let all_match = recomputed_shards
                    .iter()
                    .take(d_prime_hashes.len()) // in case D′ is shorter
                    .zip(d_prime_hashes)
                    .all(|(shard, expected_hash)| do_hash(shard) == expected_hash);

                if all_match {
                    log::info!(" M is verified and consistent, delivering...");
                    rbc_context.status = Status::TERMINATED;
                    self.terminate(reconstructed_data).await;
                    return;
                } else {
                    log::warn!(" M failed verification against D′, discarding");
                    return;
                }
            }
        }
    }
}
