// TODO: Call broadcast
use crate::{Context, ProtMsg, ShareMsg, Status};
use crypto::hash::Hash;
use network::{plaintcp::CancelHandler, Acknowledgement};
use reed_solomon_rs::fec::fec::FEC;
use reed_solomon_rs::fec::fec::*;
use tokio::time::{sleep, Duration};
use types::WrapperMsg;

impl Context {
    pub async fn ready_self(&mut self, hash: Hash, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        let status = &rbc_context.status;
        if *status != Status::READY {
            return;
        }
        // assert!(
        //     *status == Status::READY,
        //     "Ready Self: Status is not READY for instance id: {:?}",
        //     instance_id
        // );
        let fragment = rbc_context.fragment.clone();
        let _ = rbc_context;
        let msg = ShareMsg {
            share: fragment,
            hash,
            origin: self.myid,
        };
        self.handle_ready(msg, instance_id).await;
    }

    pub async fn start_ready(self: &mut Context, hash: Hash, instance_id: usize) {
        // Draft a message
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        let status = &rbc_context.status;
        if *status != Status::READY {
            return;
        }
        // assert!(
        //     *status == Status::READY,
        //     "Start Ready: Status is not READY for instance id: {:?}",
        //     instance_id
        // );
        let fragment = rbc_context.fragment.clone();
        let _ = rbc_context;
        let msg = ShareMsg {
            share: if self.byz {
                Share {
                    number: self.myid,
                    data: vec![0; fragment.data.len()],
                }
            } else {
                fragment.clone()
            },
            hash,
            origin: self.myid,
        };
        // Wrap the message in a type
        let protocol_msg = ProtMsg::Ready(msg, instance_id);

        // Sleep to simulate network delay
        // sleep(Duration::from_millis(50)).await;
        // Echo to every node the encoding corresponding to the replica id
        let sec_key_map = self.sec_key_map.clone();
        if !self.crash {
            for (replica, sec_key) in sec_key_map.into_iter() {
                if replica == self.myid {
                    self.ready_self(hash, instance_id).await;
                    continue;
                }

                let wrapper_msg =
                    WrapperMsg::new(protocol_msg.clone(), self.myid, &sec_key.as_slice());
                let cancel_handler: CancelHandler<Acknowledgement> =
                    self.net_send.send(replica, wrapper_msg).await;
                self.add_cancel_handler(cancel_handler);
            }
        }
    }

    pub async fn handle_ready(self: &mut Context, msg: ShareMsg, instance_id: usize) {
        assert!(
            msg.share.data.len() != 0,
            "Received empty share for instance id: {:?}",
            instance_id
        );
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        if rbc_context.status == Status::TERMINATED {
            return;
        }
        if rbc_context.status == Status::OUTPUT {
            let output_message = rbc_context.output_message.clone();
            rbc_context.status = Status::TERMINATED;
            let _ = rbc_context;
            log::info!("Terminating for instance id: {:?}", instance_id);
            self.terminate(output_message).await;
            return;
        }
        // log::info!("Received {:?} as ready", msg);

        let senders = rbc_context
            .ready_senders
            .entry(msg.hash.clone())
            .or_default();

        if senders.insert(msg.origin) {
            let shares = rbc_context
                .received_readys
                .entry(msg.hash.clone())
                .or_default();
            shares.push(msg.share);

            let (max_shares_count, max_shares_hash) = rbc_context.get_max_ready_count();

            // If we have enough shares for a hash, prepare for error correction
            if max_shares_count >= self.num_nodes - self.num_faults {
                if let Some(hash) = max_shares_hash {
                    let shares_for_correction = rbc_context.received_readys.get(&hash).unwrap();
                    assert!(
                        shares_for_correction.len() >= self.num_nodes - self.num_faults,
                        "Not enough shares for error correction"
                    );
                    let f = match FEC::new(self.num_faults, self.num_nodes) {
                        Ok(f) => f,
                        Err(e) => {
                            log::info!("FEC initialization failed with error: {:?}", e);
                            return;
                        }
                    };
                    // log::info!("Decoding {:?}", shares_for_correction.to_vec());
                    // assert that the length of each share for correction is the same
                    // for share in shares_for_correction.iter() {
                    //     assert!(
                    //         share.data.len() == shares_for_correction[0].data.len(),
                    //         "Share length mismatch, 0: {:?}, index: {:?}",
                    //         shares_for_correction[0].data.len(),
                    //         share.data.len()
                    //     );
                    // }
                    match f.decode([].to_vec(), shares_for_correction.to_vec()) {
                        Ok(data) => {
                            if data.len() != 0 {
                                log::info!("Outputting: for instance id: {:?}", instance_id);
                                rbc_context.output_message = data;
                                rbc_context.status = Status::OUTPUT;
                            }
                        }
                        Err(e) => {
                            log::info!("Decoding failed with error: {}", e.to_string());
                        }
                    }
                    if rbc_context.status == Status::OUTPUT {
                        let output_message = rbc_context.output_message.clone();
                        rbc_context.status = Status::TERMINATED;
                        let _ = rbc_context;
                        log::info!("Terminating for instance id: {:?}", instance_id);
                        self.terminate(output_message).await;
                        return;
                    }
                }
            }
        }
    }
}
