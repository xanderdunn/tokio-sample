// System
use std::collections::BTreeMap;
use std::sync::Arc;

// Third Party
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;
use tonic::transport::Channel;
use tonic::Status;

// Local
use super::types::{DealingValue, NodeIndex, ProtocolRoundIndex, PublicKey};
use super::utils;
use crate::sample::sample_client::SampleClient;
use crate::sample::Dealing;

pub struct Peer {
    pub address: String,
    pub public_key: PublicKey,
    // This is the tonic TLS connection
    pub connection: Option<SampleClient<Channel>>,
    // The receive_dealing() server side sends dealings here
    pub server_dealing_sender: Option<Sender<Result<Dealing, Status>>>,
    // The receive_dealing() client side sends dealings here
    pub client_dealing_sender: Option<Sender<Dealing>>,
    pub random_dealings: BTreeMap<ProtocolRoundIndex, DealingValue>,
}

#[derive(Clone)]
pub struct PeerMap {
    inner: Arc<RwLock<BTreeMap<PublicKey, Peer>>>,
}

impl PeerMap {
    pub fn new() -> Self {
        let inner = Arc::new(RwLock::new(BTreeMap::new()));
        PeerMap { inner }
    }

    pub fn peers_count(&self) -> usize {
        let lock = self.inner.read();
        return lock.len();
    }

    pub fn public_keys(&self) -> Vec<PublicKey> {
        let lock = self.inner.read();
        return lock.iter().map(|(_, v)| v.public_key.clone()).collect();
    }

    pub fn index_of_public_key(&self, public_key: PublicKey) -> NodeIndex {
        let lock = self.inner.read();
        return lock
            .iter()
            .position(|(peer_public_key, _)| peer_public_key == &public_key)
            .unwrap() as u32;
    }

    pub fn contains_public_key(&self, public_key: PublicKey) -> bool {
        let lock = self.inner.read();
        return lock.get(&public_key.clone()).is_some();
    }

    pub fn with_map<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut BTreeMap<PublicKey, Peer>) -> T,
    {
        let mut lock = self.inner.write();
        func(&mut *lock)
    }

    pub fn set_peer_server_dealing_sender(
        &self,
        peer_public_key: PublicKey,
        sender: Sender<Result<Dealing, Status>>,
    ) {
        let mut lock = self.inner.write();
        if let Some(mut peer) = lock.get_mut(&peer_public_key.clone()) {
            peer.server_dealing_sender = Some(sender);
        } else {
            panic!("Attempted to create a receive_dealings stream for a peer I don't have!");
        }
    }

    pub fn add_peer(&self, new_peer: Peer) {
        let mut lock = self.inner.write();
        // Don't add the peer if it's already there
        let public_keys: Vec<PublicKey> = lock.iter().map(|(_, v)| v.public_key.clone()).collect();
        if !lock.contains_key(&new_peer.public_key) {
            assert!(lock.get(&new_peer.public_key.clone()).is_none());
            utils::debug_line_to_file("Added Peer.", "added_peer.debug.txt");
            lock.insert(new_peer.public_key.clone(), new_peer);
        }
        if !utils::has_unique_elements(public_keys) {
            panic!("There is a duplicate public key in my peers!");
        }
        let addresses: Vec<String> = lock.iter().map(|(_, v)| v.address.clone()).collect();
        if !utils::has_unique_elements(addresses) {
            panic!("There is a duplicate address in my peers!");
        }
    }
}
