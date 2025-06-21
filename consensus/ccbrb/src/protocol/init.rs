use crate::msg::SendMsg;
use crate::Status;
use crate::{Context, ProtMsg};
use consensus::get_shards;
use crypto::hash::do_hash;
use reed_solomon_rs::fec::fec::Share;

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

        // Parameters for encoding
        let n = self.num_nodes;
        let k = self.num_faults + 1;
        let shards = get_shards(input_msg.clone(), k, n); // Vec<Vec<u8>>
        assert_eq!(shards.len(), n);

        // Compute D = [H(d₁), ..., H(dₙ)]
        let d_hashes: Vec<_> = shards.iter().map(|s| do_hash(s)).collect();

        // Create Share from our own shard
        let my_share = Share {
            number: self.myid,
            data: shards[self.myid].clone(),
        };

        rbc_context.fragment = my_share.clone();

        // Construct SendMsg
        let send_msg = SendMsg {
            id: instance_id as u64,
            d_j: my_share,
            d_hashes: d_hashes.clone(),
            origin: self.myid,
        };

        // Handle own INIT
        self.handle_init(send_msg.clone(), instance_id).await;

        // Broadcast INIT
        self.broadcast(ProtMsg::Init(send_msg, self.myid)).await;
    }

    pub async fn handle_init(&mut self, msg: SendMsg, instance_id: usize) {
        let rbc_context = self.rbc_context.entry(instance_id).or_default();

        assert_eq!(msg.d_hashes.len(), self.num_nodes);

        let computed_hash = do_hash(&msg.d_j.data);
        let expected_hash = msg.d_hashes[self.myid];

        if computed_hash != expected_hash {
            println!("Hash mismatch in INIT: ignoring");
            return;
        }

        self.start_echo(msg, instance_id).await;
    }
}
