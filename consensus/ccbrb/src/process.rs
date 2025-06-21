use std::sync::Arc;

use super::ProtMsg;
use crate::context::Context;
use crate::msg::{EchoMsg, ReadyMsg, SendMsg};
use crypto::hash::verf_mac;
use types::{SyncMsg, SyncState, WrapperMsg};

impl Context {
    pub fn check_proposal(&self, wrapper_msg: Arc<WrapperMsg<ProtMsg>>) -> bool {
        let byte_val =
            bincode::serialize(&wrapper_msg.protmsg).expect("Failed to serialize object");

        let sec_key = match self.sec_key_map.get(&wrapper_msg.sender) {
            Some(val) => val,
            None => {
                panic!("Secret key not available, this shouldn't happen")
            }
        };

        if !verf_mac(&byte_val, sec_key.as_slice(), &wrapper_msg.mac) {
            log::warn!("MAC Verification failed.");
            return false;
        }
        true
    }

    pub(crate) async fn process_msg(&mut self, wrapper_msg: WrapperMsg<ProtMsg>) {
        log::debug!("Received protocol msg: {:?}", wrapper_msg);
        let msg = Arc::new(wrapper_msg.clone());

        if self.check_proposal(msg) {
            match wrapper_msg.protmsg {
                ProtMsg::Echo(main_msg, instance_id) => {
                    log::info!(
                        "Received Echo for instance id {} from node {:?}",
                        instance_id,
                        main_msg.origin
                    );
                    self.handle_echo(main_msg, instance_id).await;
                }
                ProtMsg::Ready(main_msg, instance_id) => {
                    log::info!(
                        "Received Ready for instance id {} from node {:?}",
                        instance_id,
                        main_msg.origin
                    );
                    self.handle_ready(main_msg, instance_id).await;
                }
                ProtMsg::Init(main_msg, instance_id) => {
                    log::info!(
                        "Received Init for instance id {} from node {:?}",
                        instance_id,
                        main_msg.origin
                    );
                    self.handle_init(main_msg, instance_id).await;
                }
            }
        } else {
            log::warn!(
                "MAC Verification failed for message {:?}",
                wrapper_msg.protmsg
            );
        }
    }

    pub async fn terminate(&mut self, data: Vec<u8>) {
        let cancel_handler = self
            .sync_send
            .send(
                0,
                SyncMsg {
                    sender: self.myid,
                    state: SyncState::COMPLETED,
                    value: data,
                },
            )
            .await;
        self.add_cancel_handler(cancel_handler);
    }
}
