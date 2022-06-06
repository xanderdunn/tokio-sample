// System
use ring::{
    rand,
    signature::{self, KeyPair},
};
use std::sync::Arc;

// Third Party
use parking_lot::RwLock;

// Local
use super::types::ProtocolRoundIndex;

#[derive(Clone)]
pub struct NodeSetup {
    pub receivers: u32,
    pub ad: Vec<u8>,
    _key: Arc<signature::Ed25519KeyPair>,
    pub public_key: Vec<u8>,
    // See here for thread safe interior mutability: https://ricardomartins.cc/2016/06/25/interior-mutability-thread-safety
    // Cell and RefCell are interior mutability on a single thread only
    protocol_round: Arc<RwLock<ProtocolRoundIndex>>,
}

impl NodeSetup {
    pub fn new(receivers: u32) -> Result<Self, ()> {
        let ad: Vec<u8> = "asdfasdfasdfasdfasdfasdf".as_bytes().to_vec();

        let rng = rand::SystemRandom::new();
        let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let _key = Arc::new(signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()).unwrap());
        let public_key = _key.public_key().as_ref().to_vec();

        Ok(Self {
            receivers,
            ad,
            _key,
            public_key,
            protocol_round: Arc::new(RwLock::new(0)),
        })
    }

    pub fn get_next_round(&self) -> ProtocolRoundIndex {
        let mut round = self.protocol_round.write();
        let current_round: ProtocolRoundIndex = *round;
        *round = current_round + 1; // increment for next round
        current_round
    }
}
