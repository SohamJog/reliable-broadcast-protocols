// TODO: Make into broadcast
use crypto::hash::{do_hash, Hash};
use reed_solomon_rs::fec::fec::*;

use crate::{Context, ProtMsg, ShareMsg};

use crate::RBCState;
use types::WrapperMsg;

use crate::Status;
use network::{plaintcp::CancelHandler, Acknowledgement};
use tokio::time::{sleep, Duration};

impl Context {
    pub async fn echo_self(&mut self, hash: Hash, share: Share, instance_id: usize) {
        let msg = ShareMsg {
            share: share.clone(),
            hash,
            origin: self.myid,
        };
        self.handle_echo(msg, instance_id).await;
    }
    pub async fn start_echo(self: &mut Context, msg_content: Vec<u8>, instance_id: usize) {
        let hash = do_hash(&msg_content);
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        let status = &rbc_context.status;
        // if *status != Status::INIT || *status != Status::WAITING {
        //     return;
        // }
        // assert!(
        //     *status == Status::INIT || *status == Status::WAITING,
        //     "Start Echo: Status is not INIT for instance id: {:?}. Found {:?} instead.",
        //     instance_id, status
        // );
        let f = match FEC::new(self.num_faults, self.num_nodes) {
            Ok(f) => f,
            Err(e) => {
                log::info!("FEC initialization failed with error: {:?}", e);
                return;
            }
        };
        let mut shares: Vec<Share> = vec![
            Share {
                number: 0,
                data: vec![]
            };
            self.num_nodes
        ];
        {
            let output = |s: Share| {
                shares[s.number] = s.clone(); // deep copy
            };
            assert!(msg_content.len() > 0, "Message content is empty");
            if let Err(e) = f.encode(&msg_content, output) {
                log::info!("Encoding failed with error: {:?}", e);
            }
            //f.encode(&msg_content, output)?;
        }
        rbc_context.fragment = shares[self.myid].clone();

        // log::info!("Decoding Shares: {:?}", shares);

        // Echo to every node the encoding corresponding to the replica id
        let sec_key_map = self.sec_key_map.clone();
        // Sleep to simulate network delay
        // log::info!("Starting echo for: {:?}", instance_id,);
        // sleep(Duration::from_millis(50)).await;
        if !self.crash {
            for (replica, sec_key) in sec_key_map.into_iter() {
                if replica == self.myid {
                    self.echo_self(hash, shares[self.myid].clone(), instance_id)
                        .await;
                    continue;
                }

                let msg = ShareMsg {
                    share: if self.byz {
                        Share {
                            number: replica,
                            data: vec![],
                        }
                    } else {
                        shares[replica].clone()
                    },
                    hash,
                    origin: self.myid,
                };

                let protocol_msg = ProtMsg::Echo(msg, instance_id);
                let wrapper_msg =
                    WrapperMsg::new(protocol_msg.clone(), self.myid, &sec_key.as_slice());
                let cancel_handler: CancelHandler<Acknowledgement> =
                    self.net_send.send(replica, wrapper_msg).await;
                self.add_cancel_handler(cancel_handler);
            }
        }
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        rbc_context.status = Status::ECHO;

        let (max_count, mode_content) = rbc_context.get_max_echo_count();
        if max_count >= self.num_nodes - self.num_faults {
            //<Ready, f(your own fragment), h> to everyone
            if let Some(hash) = mode_content {
                rbc_context.status = Status::READY;
                rbc_context.sent_ready = true;
                self.start_ready(hash, instance_id).await;
            }
            // let rbc_context = self.rbc_context.entry(instance_id).or_default();
        }
        // log::info!("Broadcasted echo for: {:?}", instance_id,);
    }

    pub async fn handle_echo(self: &mut Context, msg: ShareMsg, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        let senders = rbc_context.echo_senders.entry(msg.hash).or_default();

        // Only count if we haven't seen an echo from this sender for this message
        if senders.insert(msg.origin) {
            *rbc_context.received_echo_count.entry(msg.hash).or_default() += 1;

            let (max_count, mode_content) = rbc_context.get_max_echo_count();
            // TODO: Clean
            let rbc_context = self.rbc_context.entry(instance_id).or_default();
            let status = &rbc_context.status;
            // let _ = rbc_context;
            // Check if we've received n - techoes for this message
            if max_count >= self.num_nodes - self.num_faults && *status == Status::ECHO {
                //<Ready, f(your own fragment), h> to everyone
                if let Some(hash) = mode_content {
                    rbc_context.status = Status::READY;
                    rbc_context.sent_ready = true;
                    self.start_ready(hash, instance_id).await;
                }
            }
        }
    }
}
