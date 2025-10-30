use crate::msg::{EchoMsg, SendMsg};

use crate::Status;
use crate::{Context, ProtMsg};
use bincode;
use crypto::hash::{do_hash};
use network::{plaintcp::CancelHandler, Acknowledgement};
use reed_solomon_rs::fec::fec::*;
use types::WrapperMsg;

impl Context {
    pub async fn start_echo(&mut self, msg: SendMsg, instance_id: usize) {
        let d_hashes = msg.d_hashes.clone(); // D = [H(d1), ..., H(dn)]
        let c = do_hash(&bincode::serialize(&d_hashes).unwrap()); // c = H(D)
                                                                  // log::info!(
                                                                  //     "Starting ECHO for instance_id {} with c: {:?}, d_hashes: {:?}",
                                                                  //     instance_id,
                                                                  //     c,
                                                                  //     d_hashes
                                                                  // );

        let f = match FEC::new(self.num_faults, self.num_nodes) {
            Ok(f) => f,
            Err(e) => {
                log::info!("FEC initialization failed with error: {:?}", e);
                return;
            }
        };

        let mut pi: Vec<Share> = vec![
            Share {
                number: 0,
                data: vec![]
            };
            self.num_nodes
        ];
        {
            let output = |s: Share| {
                pi[s.number] = s.clone(); // deep copy
            };
            // log::info!(
            //     "d_hashes before encoding: {:?}, instance_id: {}",
            //     d_hashes,
            //     instance_id
            // );
            assert!(d_hashes.len() > 0, "Message content is empty");
            // let encoded: Vec<u8> = d_hashes.iter().flatten().copied().collect();
            let serialized_hashes = bincode::serialize(&d_hashes).unwrap();

            if let Err(e) = f.encode(&serialized_hashes, output) {
                log::info!("Encoding failed with error: {:?}", e);
            }
            //f.encode(&msg_content, output)?;
        }
        if self.byz {
            // if byzantine, set all shares to empty, but make sure to keep the size consistent, so fill with 0
            for i in 0..self.num_nodes {
                // set p[i].data to 0, but keep the size of data consistent
                pi[i].data = vec![0; pi[i].data.len()];
            }
        }

        // log::info!(
        //     "Echo: Encoded shares for instance_id {}: {:?}",
        //     instance_id,
        //     pi
        // );

        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        rbc_context.fragment = msg.d_j.clone();

        assert!(
            rbc_context.status == Status::ECHO,
            "ECHO: Status is not ECHO for instance id: {:?}",
            instance_id
        );
        // rbc_context.status = Status::ECHO;

        if !self.crash {
            for replica in 0..self.num_nodes {
                let share = if self.byz && replica != self.myid {
                    msg.d_j.clone()
                    // Share {
                    //     number: replica,
                    //     data: vec![],
                    // }
                } else {
                    msg.d_j.clone()
                };
                // send ⟨𝑖𝑑, ECHO, (𝑑𝑖, 𝜋𝑗, 𝑐)⟩ to node 𝑗
                let echo_msg = EchoMsg {
                    id: instance_id as u64,
                    d_i: share,
                    pi_i: pi[replica].clone(), // πj
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
    }

    pub async fn handle_echo(&mut self, echo_msg: EchoMsg, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        // Serialize πᵢ
        let pi_i_serialized = bincode::serialize(&echo_msg.pi_i).unwrap();

        // Track senders per (c, πᵢ)
        let pi_i_map = rbc_context.echo_senders.entry(echo_msg.c).or_default();
        let senders = pi_i_map.entry(pi_i_serialized.clone()).or_default();

        if !senders.insert(echo_msg.origin) {
            return; // duplicate
        }

        // Store dᵢ
        let data_entry = rbc_context
            .fragments_data
            .entry((instance_id as u64, echo_msg.c))
            .or_default();
        data_entry.push(echo_msg.d_i.clone());

        // Check if 2t + 1 ECHOs for same (c, πᵢ)
        if senders.len() >= 2 * self.num_faults + 1 && rbc_context.status == Status::ECHO {
            rbc_context.status = Status::READY;
            rbc_context.sent_ready = true;
            //send pi i if byzantine, otherwise clear the data of pi_i
            let share = {
                if !self.byz {
                    echo_msg.pi_i.clone()
                } else {
                    Share {
                        number: echo_msg.pi_i.number,
                        data: echo_msg.pi_i.data.iter().map(|_| 0).collect(),
                    }
                }
            };
            self.start_ready(echo_msg.c, share, instance_id).await;
        }
    }
}
