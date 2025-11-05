use crate::msg::SendMsg;
use crate::Status;
use crate::{Context, ProtMsg};
use consensus::get_shards;
use crypto::hash::do_hash;
use reed_solomon_rs::fec::fec::Share;
use types::WrapperMsg;

impl Context {
    pub async fn start_init(&mut self, input_msg: Vec<u8>, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();
        let status = &rbc_context.status;

        assert!(
            *status == Status::WAITING,
            "INIT: Status is not WAITING for instance id: {:?}",
            instance_id
        );
        rbc_context.status = Status::INIT;

        let n = self.num_nodes;
        let k = self.num_faults + 1;
        // d
        let shards = get_shards(input_msg.clone(), k, n - k);
        assert_eq!(shards.len(), n);

        // print input message and shards. input message and shards for instance_id:
        // log::info!(
        //     "INIT: Input message for instance_id {}: {:?}",
        //     instance_id,
        //     input_msg
        // );
        // log::info!("INIT: Shards for instance_id {}: {:?}", instance_id, shards);

        // D
        let d_hashes: Vec<_> = shards.iter().map(|s| do_hash(s)).collect();

        // log::info!(
        //     "INIT: D hashes for instance_id {}: {:?}",
        //     instance_id,
        //     d_hashes
        // );

        // Store our own share
        let my_share = Share {
            number: self.myid,
            data: shards[self.myid].clone(),
        };
        rbc_context.fragment = my_share.clone();

        // Send ourselves our own message
        let my_msg = SendMsg {
            id: instance_id as u64,
            d_j: my_share,
            d_hashes: d_hashes.clone(),
            origin: self.myid,
        };
        self.handle_init(my_msg.clone(), instance_id).await;

        // Send correct share to each replica
        for (replica, sec_key) in self.sec_key_map.clone() {
            if replica == self.myid {
                continue;
            }

            let share = if self.byz {
                // If we're Byzantine, corrupt the share
                Share {
                    number: replica,
                    data: shards[replica].clone(),
                    //data: vec![0; shards[replica].len()],
                }
            } else {
                Share {
                    number: replica,
                    data: shards[replica].clone(),
                }
            };

            let send_msg = SendMsg {
                id: instance_id as u64,
                d_j: share,
                d_hashes: d_hashes.clone(),
                origin: self.myid,
            };

            let protmsg = ProtMsg::Init(send_msg, instance_id);
            let wrapper = WrapperMsg::new(protmsg, self.myid, &sec_key);
            let cancel_handler = self.net_send.send(replica, wrapper).await;
            self.add_cancel_handler(cancel_handler);
        }
    }

    pub async fn handle_init(&mut self, msg: SendMsg, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        assert_eq!(msg.d_hashes.len(), self.num_nodes);

        // H(di)
        let computed_hash = do_hash(&msg.d_j.data);
        // Di
        let expected_hash = msg.d_hashes[self.myid];

        if computed_hash != expected_hash {
            log::info!("Hash mismatch in INIT: ignoring.");
            // log::info!(
            //     "Computed hash: {:?}, Expected hashes: {:?}, instance_id: {}",
            //     computed_hash,
            //     msg.d_hashes,
            //     instance_id
            // );

            return;
        }
        // let &mut status = &rbc_context.status;
        if (rbc_context.status == Status::INIT || rbc_context.status == Status::WAITING)
            && !self.crash
        {
            rbc_context.status = Status::ECHO;
            self.start_echo(msg, instance_id).await;
        }
    }
}
