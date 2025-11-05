use crate::{
    msg::{ProtMsg, ReadyMsg},
    Context, Status,
};
use bincode;
use consensus::{reconstruct_data};
use crypto::hash::{do_hash, Hash};

use reed_solomon_rs::fec::fec::{Share, FEC};
use std::collections::HashSet;
use types::Replica;
use types::WrapperMsg;

impl Context {
    pub async fn start_ready(&mut self, c: Hash, pi_i: Share, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        if rbc_context.status != Status::READY {
            return;
        }

        let ready_msg = ReadyMsg {
            id: instance_id as u64,
            c,
            pi_i: pi_i.clone(),
            origin: self.myid,
        };

        let proto = ProtMsg::Ready(ready_msg.clone(), instance_id);
        if self.crash {
            return;
        }
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
        // log::info!(
        //     "Handling ready message: {:?} for instance_id: {}",
        //     msg,
        //     instance_id
        // );
        let mut cancel_handlers = vec![];

        {
            let rbc_context = self.rbc_context.entry(instance_id).or_default();
            if rbc_context.status == Status::TERMINATED {
                return;
            }

            let pi_i = msg.pi_i.clone();

            let pi_i_serialized = bincode::serialize(&pi_i).unwrap();
            // log::info!(
            //     "Ready message received for instance_id: {}, c: {:?}, pi_i: {:?}, origin: {}",
            //     instance_id,
            //     msg.c,
            //     pi_i,
            //     msg.origin
            // );
            // Track senders per (c, πᵢ)
            let pi_i_map = rbc_context.ready_senders.entry(msg.c).or_default();
            let senders = pi_i_map.entry(pi_i_serialized.clone()).or_default();

            if !senders.insert(msg.origin) {
                // log::debug!(
                //     "Duplicate READY message received for instance_id: {}, c: {:?}, pi_i: {:?}, origin: {}",
                //     instance_id,
                //     msg.c,
                //     pi_i,
                //     msg.origin
                // );
                return; // duplicate
            }

            let hashes_entry = rbc_context
                .fragments_hashes
                .entry((instance_id as u64, msg.c))
                .or_default();
            hashes_entry.push(pi_i.clone());

            log::debug!("About to process ready");
            //if (not yet sent ⟨𝑖𝑑, READY, 𝑐⟩ and received 𝑡 + 1 ⟨READY⟩ messages with the same 𝑐) then

            if !rbc_context.sent_ready {
                // log::info!("Not sent ready yet, instance_id: {}", instance_id);
                let threshold = self.num_faults + 1;
                let mut all_ready_senders: HashSet<Replica> = HashSet::new();
                for senders in pi_i_map.values() {
                    all_ready_senders.extend(senders.iter().copied());
                }

                if all_ready_senders.len() >= threshold {
                    if let Some(echo_map) = rbc_context.echo_senders.get(&msg.c) {
                        for (pi_i_bytes, _echo_senders) in echo_map {
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
            // log::info!(
            //     "Received enough hash shares for instance_id: {}, c: {:?}, count: {}",
            //     instance_id,
            //     msg.c,
            //     hash_shares.len()
            // );
            // check if the length of data of all shares is consistent
            let data_length = hash_shares[0].data.len();
            if !hash_shares
                .iter()
                .all(|share| share.data.len() == data_length)
            {
                log::warn!("Inconsistent data lengths in hash shares, cannot proceed");
                return;
            }

            let f = FEC::new(self.num_faults, self.num_nodes).unwrap();
            // log::info!(
            //     "About to decode D′ for instance_id: {}, c: {:?}, hash_shares: {:?}",
            //     instance_id,
            //     msg.c,
            //     hash_shares.clone()
            // );

            let mut d_prime = match f.decode(vec![], hash_shares.clone()) {
                Ok(data) => data,
                Err(_) => {
                    log::warn!("Could not reconstruct D′ from hash shares, trying higher error tolerance later");
                    return;
                }
            };

            // while d prime is not empty and the last element is 95, pop the last element
            while !d_prime.is_empty() && d_prime.last() == Some(&95) {
                d_prime.pop();
            }

            // log do hash d prime and msg.c
            // log::info!(
            //     "D′ decoded for instance_id: {}, c: {:?}, d_prime: {:?}",
            //     instance_id,
            //     msg.c,
            //     &d_prime
            // );
            // log::info!(
            //     "Comparing hash of d prime and msg.c for instance_id: {}, c: {:?}, d_prime: {:?}",
            //     instance_id,
            //     msg.c,
            //     do_hash(&d_prime)
            // );

            // if 𝐻(𝐷′) = 𝑐 then
            if do_hash(&d_prime) == msg.c {
                // log::info!(
                //     "D′ matches c for instance_id: {}, c: {:?}",
                //     instance_id,
                //     msg.c
                // );

                let d_hashes: Vec<Hash> = match bincode::deserialize(&d_prime) {
                    Ok(decoded) => decoded,
                    Err(e) => {
                        log::warn!("Failed to deserialize D′ into d_hashes: {:?}", e);
                        return;
                    }
                };

                // log::info!(
                //     "after decoding D′ hashes: {:?} for instance_id: {}, c: {:?}",
                //     d_hashes,
                //     instance_id,
                //     msg.c
                // );
                let valid_hashes: HashSet<Hash> = d_hashes.into_iter().collect();

                let data_shares = rbc_context
                    .fragments_data
                    .get(&(instance_id as u64, msg.c))
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|share| valid_hashes.contains(&do_hash(&share.data)))
                    .collect::<Vec<_>>();

                // wait for t+1 ⟨ECHO⟩ message where 𝐻(𝑑𝑗) ∈ 𝐷′and filter 𝑓𝑟𝑎𝑔𝑚𝑒𝑛𝑡𝑠𝑑𝑎𝑡𝑎[(𝑖𝑑, 𝑐)] accordingly
                // log::info!(
                //     "Data shares count after filtering: {} for instance_id: {}, c: {:?}",
                //     data_shares.len(),
                //     instance_id,
                //     msg.c
                // );
                if data_shares.len() < self.num_faults + 1 {
                    return; // wait for more
                }

                // print actual data shares
                // log::info!(
                //     "Data shares for instance_id: {}, c: {:?}: {:?}",
                //     instance_id,
                //     msg.c,
                //     data_shares
                // );

                let mut input_shares: Vec<Option<Vec<u8>>> = vec![None; self.num_nodes];

                for share in &data_shares {
                    input_shares[share.number] = Some(share.data.clone());
                }
                // log::info!(
                //     "Input shares initialized with {:?} slots for instance_id: {}, c: {:?}",
                //     input_shares,
                //     instance_id,
                //     msg.c
                // );

                let n = self.num_nodes;
                let k = self.num_faults + 1;

                if reconstruct_data(&mut input_shares, k, n - k).is_err() {
                    log::warn!("reconstruct_data failed");
                    return;
                }

                // let mut reconstructed_data = vec![];
                // for maybe in input_shares
                //     .iter()
                //     .take(self.num_nodes - self.num_faults - 1)
                // {
                //     if let Some(ref block) = maybe {
                //         reconstructed_data.extend_from_slice(block);
                //     }
                // }

                // let recomputed_shards = get_shards(&mut input_shares, k, n - k);
                let recomputed_shards = input_shares
                    .into_iter()
                    .map(|maybe| {
                        maybe.unwrap_or_else(|| {
                            log::warn!("Missing data in input shares, returning empty shard");
                            vec![]
                        })
                    })
                    .collect::<Vec<_>>();

                let d_prime_hashes: Vec<Hash> = bincode::deserialize(&d_prime).unwrap();

                // log::info!(
                //     "Recomputed shards: {:?} for instance_id: {}, c: {:?}",
                //     recomputed_shards,
                //     instance_id,
                //     msg.c
                // );

                let recomputed_hashes: Vec<Hash> = recomputed_shards
                    .iter()
                    .map(|shard| do_hash(shard))
                    .collect();

                // log::info!(
                //     "after D′ hashes: {:?}, recomputed hashes: {:?}",
                //     d_prime_hashes,
                //     recomputed_hashes
                // );

                let all_match = recomputed_hashes == d_prime_hashes;

                if all_match {
                    log::info!(
                        " M is verified and consistent, delivering... for instance_id: {}",
                        instance_id
                    );
                    rbc_context.status = Status::TERMINATED;
                    let output_message = recomputed_shards.concat();
                    log::info!(
                        "RBC instance {} successfully terminated with output message of length {}",
                        instance_id,
                        output_message.len()
                    );
                    self.terminate(instance_id, output_message).await;
                    return;
                } else {
                    log::warn!(" M failed verification against D′, discarding");
                    // empty Vec<u8>
                    // TEMPORARY ASSERT
                    assert!(instance_id / 10000 < self.num_faults);
                    rbc_context.status = Status::TERMINATED;
                    let empty_output: Vec<u8> = vec![];
                    self.terminate(instance_id, empty_output).await; // bottom
                    return;
                }
            } else {
                //log warn H(d prime ) != c
                log::warn!(
                    "H(D′) does not match c for instance_id: {}, c: {:?}",
                    instance_id,
                    msg.c
                );
            }
        }
    }
}
