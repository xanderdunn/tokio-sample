// System
use std::collections::{BTreeMap, HashMap};

// Third Party
use backoff::{future::retry, ExponentialBackoff};
use ring::{rand, signature};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::metadata::BinaryMetadataValue;
use tonic::Code;
use tonic::{Request, Response, Status};

// Local
use super::peer::{Peer, PeerMap};
use super::types::{DealingValue, NodeIndex, ProtocolRoundIndex, PublicKey};
use super::utils;
use crate::node_setup::NodeSetup;
use crate::sample::sample_client::SampleClient;
use crate::sample::sample_server::Sample;
use crate::sample::{
    AddPeerRequest, Dealing, HealthRequest, HealthResponse, IteratePeersRequest, PeerResponse,
    SharingRequest, SharingResponse,
};

// Our gRPC server
pub struct MySample {
    peers: PeerMap,
    node_setup: NodeSetup,
    node_count: u32, // the total number of nodes in the network
    _hostname: String,
    // This aggregates all new dealings from all sources
    inbound_dealing_sender: broadcast::Sender<Dealing>,
}

impl MySample {
    pub fn new(node_count: u32, hostname: String) -> Self {
        let node_setup = NodeSetup::new(node_count).unwrap();
        let peers = PeerMap::new();

        // Add myself to the peers map so that all dealings can be conveniently iterated.
        let self_peer: Peer = Peer {
            address: "http://localhost:2323".to_string(),
            public_key: node_setup.public_key.clone(),
            connection: None,
            server_dealing_sender: None,
            client_dealing_sender: None,
            random_dealings: BTreeMap::new(),
        };
        peers.add_peer(self_peer, node_count);

        // inbound_dealing_channel
        // Aggregate all inbound dealings
        // These dealings come from: client-side streams, server-side streams, and my own created
        // dealings
        // This channel should stay open forever, always waiting for inbound dealings
        let (inbound_dealing_sender, mut inbound_dealing_receiver): (
            broadcast::Sender<Dealing>,
            broadcast::Receiver<Dealing>,
        ) = broadcast::channel(1000);
        let node_setup_to_move = node_setup.clone();
        tokio::spawn(async move {
            let mut dealings_aggregator: HashMap<ProtocolRoundIndex, BTreeMap<PublicKey, Dealing>> =
                HashMap::new();
            loop {
                let dealing: Dealing = inbound_dealing_receiver.recv().await.unwrap();
                let round_dealings = dealings_aggregator
                    .entry(dealing.clone().protocol_round as usize)
                    .or_insert_with(BTreeMap::new);
                round_dealings.insert(dealing.clone().public_key, dealing.clone());
                if round_dealings.len() == node_count as usize {
                    let node_setup = node_setup_to_move.clone();
                    let dealings = dealings_aggregator
                        .get(&(dealing.clone().protocol_round as usize))
                        .unwrap()
                        .clone();
                    tokio::task::spawn_blocking(move || {
                        Self::handle_received_dealings(&dealings, node_count, &node_setup);
                    });
                }
            }
        });

        Self {
            peers,
            node_setup,
            node_count,
            _hostname: hostname,
            inbound_dealing_sender,
        }
    }

    // Produce one random dealing from this node to all of my peers
    fn dealing(
        node_setup: &NodeSetup,
        _public_keys: &[PublicKey],
        _my_node_index: NodeIndex,
        node_count: u32,
    ) -> (DealingValue, ProtocolRoundIndex) {
        // Simulate some computationally intensive work
        const MESSAGE: &[u8] = b"hello, world";
        let mut sig: Vec<u8> = Vec::new();
        for _ in 0..node_count {
            for _ in 0..node_count {
                let rng = rand::SystemRandom::new();
                let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
                let key_pair = signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()).unwrap();
                sig = key_pair.sign(MESSAGE).as_ref().to_vec();
            }
        }
        let protocol_round = node_setup.get_next_round();
        (sig, protocol_round)
    }

    fn dealing_random(
        node_setup: &NodeSetup,
        public_keys: &[PublicKey],
        my_node_index: NodeIndex,
        node_count: u32,
    ) -> (DealingValue, ProtocolRoundIndex) {
        Self::dealing(node_setup, public_keys, my_node_index, node_count)
    }

    // This should be called only inside a tokio::task::spawn_blocking because it does some computationally
    // expensive work
    fn handle_received_dealings(
        dealings: &BTreeMap<PublicKey, Dealing>,
        node_count: u32,
        _node_setup: &NodeSetup,
    ) {
        let dealings: BTreeMap<NodeIndex, DealingValue> = dealings
            .iter()
            .zip(0..node_count)
            .map(|((_, dealing), node_index)| (node_index, dealing.dealing.clone()))
            .collect();
        let node_count = node_count as usize;
        assert!(dealings.len() == node_count);
        // Simulate some computationally intensive work
        const MESSAGE: &[u8] = b"hello, world";
        let mut _sig: Vec<u8> = Vec::new();
        for _ in 0..node_count {
            for _ in 0..node_count {
                let rng = rand::SystemRandom::new();
                let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
                let key_pair = signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()).unwrap();
                _sig = key_pair.sign(MESSAGE).as_ref().to_vec();
            }
        }
        utils::debug_line_to_file("Done.", "opening_complete.debug.txt");
    }
}

// implementing rpc for service defined in .proto
#[tonic::async_trait]
impl Sample for MySample {
    async fn initial_dealing(
        &self,
        _request: Request<SharingRequest>,
    ) -> Result<Response<SharingResponse>, Status> {
        let public_keys: Vec<PublicKey> = self.peers.public_keys();
        for _ in 0..=2 {
            // Create key, kappa, and lambda
            // create new dealings and queue them for broadcast
            // The presignature is kappa
            let node_setup = self.node_setup.clone();
            let node_count = self.node_count;
            let my_node_index = self
                .peers
                .index_of_public_key(node_setup.public_key.clone());
            let public_keys = public_keys.clone();
            let peers = self.peers.clone();
            let inbound_dealing_sender = self.inbound_dealing_sender.clone();
            tokio::task::spawn_blocking(move || {
                let (dealing, protocol_round) = Self::dealing_random(
                    &node_setup.clone(),
                    &public_keys,
                    my_node_index as u32,
                    node_count,
                );
                utils::debug_line_to_file("Created.", "dealing_created.debug.txt");
                // Add the new key to myself
                let dealing_message = Dealing {
                    dealing: dealing,
                    protocol_round: protocol_round as u32,
                    public_key: node_setup.public_key.clone(),
                };
                inbound_dealing_sender
                    .send(dealing_message.clone())
                    .unwrap();
                peers.with_map(|peers| {
                    for (_, peer) in peers.iter() {
                        // I already have my dealing. Send the new dealing across all peer streams
                        if peer.public_key != node_setup.public_key.clone() {
                            #[allow(clippy::option_if_let_else)]
                            if let Some(client_dealing_sender) = peer.client_dealing_sender.clone()
                            {
                                client_dealing_sender
                                    .blocking_send(dealing_message.clone())
                                    .unwrap();
                                utils::debug_line_to_file("Sent.", "dealing_sent.debug.txt");
                            } else if let Some(server_dealing_sender) =
                                peer.server_dealing_sender.clone()
                            {
                                server_dealing_sender
                                    .blocking_send(Ok(dealing_message.clone()))
                                    .unwrap();
                                utils::debug_line_to_file("Sent.", "dealing_sent.debug.txt");
                            } else {
                                panic!("Nowhere to send a dealing to this peer");
                            }
                        }
                    }
                });
            });
        }

        utils::debug_line_to_file("Spawned.", "spawned_all_dealing_requests.debug.txt");
        Ok(Response::new(SharingResponse {
            success: true,
            public_key: self.node_setup.public_key.clone(),
        }))
    }
    type ReceiveDealingsStream = ReceiverStream<Result<Dealing, tonic::Status>>;
    // Call this once to open a bidirectional stream for dealings
    async fn receive_dealings(
        &self,
        request: Request<tonic::Streaming<Dealing>>,
    ) -> Result<Response<Self::ReceiveDealingsStream>, Status> {
        // FIXME: In production I want to check the certificate of the sender to verify it is who
        // I think it is rather than relying on this metadata hack
        let peer_public_key: Vec<u8> = request
            .metadata()
            .get_bin("trace-proto-bin")
            .unwrap()
            .to_bytes()
            .unwrap()
            .to_vec();
        //If I don't have this peer in my Peers, add it
        if !self.peers.contains_public_key(peer_public_key.clone()) {
            let new_peer: Peer = Peer {
                address: request.remote_addr().unwrap().to_string(),
                public_key: peer_public_key.clone(),
                connection: None,
                server_dealing_sender: None,
                client_dealing_sender: None,
                random_dealings: BTreeMap::new(),
            };
            self.peers.add_peer(new_peer, self.node_count);
        }
        let mut streamer = request.into_inner();
        let (dealing_received_sender, dealing_received_receiver) = mpsc::channel(1000);
        self.peers
            .set_peer_server_dealing_sender(peer_public_key, dealing_received_sender);
        let inbound_dealing_sender = self.inbound_dealing_sender.clone();
        // server_dealing_channel
        // This channel handles server-side dealings sent from other peers
        tokio::spawn(async move {
            while let Some(request_inner) = streamer.message().await.unwrap() {
                let public_key = request_inner.public_key;
                let protocol_round: ProtocolRoundIndex = request_inner.protocol_round as usize;
                let dealing = Dealing {
                    dealing: request_inner.dealing.clone(),
                    protocol_round: protocol_round as u32,
                    public_key,
                };
                utils::debug_line_to_file("Received.", "inbound_dealing_received.debug.txt");
                inbound_dealing_sender.send(dealing).unwrap();
            }
        });
        Ok(Response::new(ReceiverStream::new(
            dealing_received_receiver,
        )))
    }
    async fn add_peer(
        &self,
        request: Request<AddPeerRequest>,
    ) -> Result<Response<PeerResponse>, Status> {
        //let remote_addr = request.remote_addr().unwrap();
        let request_inner = request.into_inner();
        let address = request_inner.address;
        // Make a health check to confirm we can connect before adding a peer
        // TODO: Make sure the address resolves to remote_addr
        let client = retry(ExponentialBackoff::default(), || async {
            Ok(SampleClient::connect(address.clone()).await?)
        })
        .await
        .unwrap();

        let request = tonic::Request::new(HealthRequest {});
        let response = client.clone().check_health(request).await?;
        let response_inner = response.into_inner();
        let public_key = response_inner.public_key;
        if !response_inner.healthy {
            println!("New peer {} returned a false healthy status", address);
            return Err(Status::new(
                Code::Aborted,
                format!("New peer {} returned a false healthy status.", address),
            ));
        }

        // Call receive_dealings() on the connection I just created to open
        // the streams that listen for dealings
        let node_setup = self.node_setup.clone();
        let client_to_move = client.clone();
        let (client_dealing_sender, mut client_dealing_receiver) = mpsc::channel(1000);
        let inbound_dealing_sender = self.inbound_dealing_sender.clone();
        // client_dealing_channel
        // This channel handles client-side dealings sent from other peers
        tokio::spawn(async move {
            let outbound = async_stream::stream! {
                loop {
                    let dealing: Dealing = client_dealing_receiver.recv().await.unwrap();
                    yield dealing;
                }
            };
            let mut request = Request::new(outbound);
            let metadata_value = BinaryMetadataValue::from_bytes(&node_setup.public_key);
            request
                .metadata_mut()
                .insert_bin("trace-proto-bin", metadata_value);
            let response = client_to_move
                .clone()
                .receive_dealings(request)
                .await
                .unwrap();
            let mut inbound = response.into_inner();
            while let Some(dealing) = inbound.message().await.unwrap() {
                utils::debug_line_to_file("Received.", "inbound_dealing_received.debug.txt");
                inbound_dealing_sender.send(dealing).unwrap();
            }
        });

        let new_peer = Peer {
            address: address.clone(),
            public_key: public_key.clone(),
            connection: Some(client.clone()),
            server_dealing_sender: None,
            client_dealing_sender: Some(client_dealing_sender),
            random_dealings: BTreeMap::new(),
        };
        // Don't add the peer if it's already there
        // Don't add the peer if it resolves to this node
        if self.node_setup.public_key != public_key {
            self.peers.add_peer(new_peer, self.node_count);
        }

        Ok(Response::new(PeerResponse {
            success: true,
            public_key: self.node_setup.public_key.clone(),
        }))
    }
    // Add as a peer every node whose ID is less than mine.
    // This is a function for setting up a network for testing with the run.sh set of Docker
    // containers
    async fn iterate_peers(
        &self,
        request: Request<IteratePeersRequest>,
    ) -> Result<Response<PeerResponse>, Status> {
        let n = request.into_inner().node_index - 1;
        let my_public_key = self.node_setup.public_key.clone();
        for n in 1..=n {
            let add_peer_request = Request::new(AddPeerRequest {
                address: format!("http://tokio-sample-node-{}:2323", n),
                public_key: my_public_key.clone(),
            });
            self.add_peer(add_peer_request).await.unwrap();
        }
        Ok(Response::new(PeerResponse {
            success: true,
            public_key: self.node_setup.public_key.clone(),
        }))
    }
    async fn check_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            healthy: true,
            public_key: self.node_setup.public_key.clone(),
        }))
    }
}
