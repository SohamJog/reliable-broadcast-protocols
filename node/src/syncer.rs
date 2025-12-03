use std::{
    collections::{HashMap, HashSet},
    net::{SocketAddr, SocketAddrV4},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use fnv::FnvHashMap;
use network::{
    plaintcp::{CancelHandler, TcpReceiver, TcpReliableSender},
    Acknowledgement,
};

use serde::{Deserialize, Serialize};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver},
        oneshot,
    },
    time,
};
use types::{Replica, SyncMsg, SyncState};

use crate::SyncHandler;

pub struct Syncer {
    pub num_nodes: usize,
    pub ready_for_broadcast: bool,

    pub rbc_id: usize,
    pub rbc_msgs: HashMap<usize, String>,
    pub rbc_start_times: HashMap<usize, u128>,
    pub rbc_complete_times: HashMap<usize, HashMap<Replica, u128>>,
    pub rbc_comp_values: HashMap<usize, HashSet<Vec<u8>>>,

    pub broadcast_msgs: Vec<u8>,

    pub sharing_complete_times: HashMap<Replica, u128>,
    pub recon_start_time: u128,
    pub net_map: FnvHashMap<Replica, String>,
    pub alive: HashSet<Replica>,
    pub timings: HashMap<Replica, u128>,

    pub cli_addr: SocketAddr,

    pub rx_net: UnboundedReceiver<SyncMsg>,
    pub net_send: TcpReliableSender<Replica, SyncMsg, Acknowledgement>,

    exit_rx: oneshot::Receiver<()>,
    /// Cancel Handlers
    pub cancel_handlers: Vec<CancelHandler<Acknowledgement>>,
}

impl Syncer {
    pub fn spawn(
        net_map: FnvHashMap<Replica, String>,
        cli_addr: SocketAddr,
        rbc_msg_size: u64,
    ) -> anyhow::Result<oneshot::Sender<()>> {
        let (exit_tx, exit_rx) = oneshot::channel();
        let (tx_net_to_server, rx_net_to_server) = unbounded_channel();
        let cli_addr_sock = cli_addr.port();
        let new_sock_address = SocketAddrV4::new("0.0.0.0".parse().unwrap(), cli_addr_sock);
        TcpReceiver::<Acknowledgement, SyncMsg, _>::spawn(
            std::net::SocketAddr::V4(new_sock_address),
            SyncHandler::new(tx_net_to_server),
        );
        
        let mut broadcast_msgs = Vec::new();
        for _ in 0..rbc_msg_size{
            broadcast_msgs.push(0 as u8);
        }

        log::info!("Requesting each party to broadcast messages of size {} bytes", rbc_msg_size);
        let mut server_addrs: FnvHashMap<Replica, SocketAddr> = FnvHashMap::default();
        for (replica, address) in net_map.iter() {
            let address: SocketAddr = address.parse().expect("Unable to parse address");
            server_addrs.insert(*replica, SocketAddr::from(address.clone()));
        }
        let net_send =
            TcpReliableSender::<Replica, SyncMsg, Acknowledgement>::with_peers(server_addrs);
        tokio::spawn(async move {
            let mut syncer = Syncer {
                net_map: net_map.clone(),
                ready_for_broadcast: false,

                rbc_id: 0,
                rbc_msgs: HashMap::default(),
                rbc_start_times: HashMap::default(),
                rbc_complete_times: HashMap::default(),
                rbc_comp_values: HashMap::default(),

                broadcast_msgs: broadcast_msgs,

                sharing_complete_times: HashMap::default(),
                recon_start_time: 0,
                num_nodes: net_map.len(),
                alive: HashSet::default(),

                timings: HashMap::default(),
                cli_addr: cli_addr,
                rx_net: rx_net_to_server,
                net_send: net_send,
                exit_rx: exit_rx,
                cancel_handlers: Vec::new(),
            };
            if let Err(e) = syncer.run().await {
                log::error!("Consensus error: {}", e);
            }
        });
        Ok(exit_tx)
    }
    pub async fn broadcast(&mut self, sync_msg: SyncMsg) {
        for replica in 0..self.num_nodes {
            let cancel_handler: CancelHandler<Acknowledgement> =
                self.net_send.send(replica, sync_msg.clone()).await;
            self.add_cancel_handler(cancel_handler);
            log::info!("Sent {:?} message to node {}", sync_msg.state, replica);
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut interval = time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                // Receive exit handlers
                exit_val = &mut self.exit_rx => {
                    exit_val.map_err(anyhow::Error::new)?;
                    log::info!("Termination signal received by the server. Exiting.");
                    break
                },
                msg = self.rx_net.recv() => {
                    // Received a protocol message
                    // Received a protocol message
                    // log::trace!("Got a message from the server: {:?}", msg);
                    let msg = msg.ok_or_else(||
                        anyhow!("Networking layer has closed")
                    )?;
                    match msg.state{
                        SyncState::ALIVE=>{
                            log::debug!("Got ALIVE message from node {}",msg.sender);
                            self.alive.insert(msg.sender);
                            if self.alive.len() == self.num_nodes{
                                self.ready_for_broadcast = true;
                            }
                        },
                        SyncState::STARTED=>{
                            log::debug!("Node {} started the protocol",msg.sender);
                        },
                        SyncState::COMPLETED=>{
                            // log::info!("Got COMPLETED message from node {} with value {:?}",msg.sender, msg.value.clone());

                            // deserialize message
                            let rbc_msg: RBCSyncMsg = bincode::deserialize(&msg.value).expect("Unable to deserialize message received from node");


                            let latency_map = self.rbc_complete_times.entry(0).or_default();
                            let _len = latency_map.len();
                            latency_map.insert(msg.sender, SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis());

                        //   let old = latency_map.get(&msg.sender).cloned();
                        //     let new = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                        //     let newly_changed = old.map_or(true, |v| v != new);
                        //     latency_map.insert(msg.sender, new);

                            let _len_new = latency_map.len();

                            // assert!(_len_new != _len, " Terminating... Sender: {}. latency_map: {:?}", msg.sender, latency_map);
                            // log::info!("ID: {}, Sender: {}, Latency map: {:?}", rbc_msg.id, msg.sender,  latency_map);


                            let value_set = self.rbc_comp_values.entry(0).or_default();
                            value_set.insert(rbc_msg.msg);
                            if latency_map.len() == self.num_nodes{

                                self.ready_for_broadcast = true;

                                if self.rbc_start_times.get(&0).is_none(){
                                    log::error!("Missing start time for RBC id {}", rbc_msg.id);
                                    continue;
                                }
                                let start_time = self.rbc_start_times
                                .get(&0)
                                .expect(&format!("Missing start time for RBC id {}", rbc_msg.id));
                                log::info!("start time: {:?}, msg id: {}",start_time, rbc_msg.id);
                                // All nodes terminated protocol

                                let mut vec_times = Vec::new();
                                for (_rep,time) in latency_map.iter(){
                                    vec_times.push(time.clone()-start_time);
                                }

                                vec_times.sort();

                                if value_set.len() > 1{
                                    log::info!("Received multiple values from nodes, broadcast failed, rerun test {:?}",value_set);
                                }
                                else{
                                    log::info!("All n nodes completed the protocol for ID: {} with latency {:?} ", rbc_msg.id,vec_times);
                                }
                                if self.rbc_id >= self.num_nodes * self.broadcast_msgs.len(){
                                    self.broadcast(SyncMsg { sender: self.num_nodes, state: SyncState::STOP, value:"".to_string().into_bytes()}).await;
                                }
                            }
                        }
                        _=>{}
                    }
                },
                _ = interval.tick() => {
                    if self.ready_for_broadcast{
                        // Initiate new broadcast
                        if self.rbc_id >= 1{
                            continue;
                        }
                        self.ready_for_broadcast = false;

                        self.rbc_id += 1;
                        // let sync_rbc_msg = RBCSyncMsg{
                        //     id: self.rbc_id,
                        //     msg: self.broadcast_msgs.get(&self.rbc_id-1).unwrap().to_string(),
                        // };
                        // let binaryfy_val = bincode::serialize(&sync_rbc_msg).expect("Failed to serialize client message");
                        // let cancel_handler:CancelHandler<Acknowledgement> = self.net_send.send(0, SyncMsg {
                        //     sender: self.num_nodes,
                        //     state: SyncState::START,
                        //     value:binaryfy_val
                        // }).await;
                        // self.add_cancel_handler(cancel_handler);


                        // self.broadcast(SyncMsg {
                        //     sender: self.num_nodes,
                        //     state: SyncState::START,
                        //     value: binaryfy_val
                        // }).await;
                        for replica in 0..self.num_nodes {
                            // COMMENT/ UNCOMMENT THIS TO DEBUG
                            // if replica != self.num_nodes - 1 || self.rbc_id != 1 {
                            //     // Skip the client node
                            //     continue;
                            // }

                            let msg_id = 0;
                            let sync_rbc_msg = RBCSyncMsg{
                                id: msg_id,
                                msg: self.broadcast_msgs.clone(),
                            };
                            let binaryfy_val = bincode::serialize(&sync_rbc_msg).expect("Failed to serialize client message");
                            let cancel_handler:CancelHandler<Acknowledgement> = self.net_send.send(replica, SyncMsg {
                                sender: self.num_nodes,
                                state: SyncState::START,
                                value:binaryfy_val
                            }).await;

                            self.add_cancel_handler(cancel_handler);
                            log::info!("Sent START message to node {}", replica);

                            
                        }
                        let start_time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        self.rbc_start_times.insert(0, start_time);
                    }
                }
            }
        }
        Ok(())
    }
    pub fn add_cancel_handler(&mut self, canc: CancelHandler<Acknowledgement>) {
        self.cancel_handlers.push(canc);
    }
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct RBCSyncMsg {
    pub id: usize,
    pub msg: Vec<u8>,
}