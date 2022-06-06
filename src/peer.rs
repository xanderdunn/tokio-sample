// System
use std::collections::BTreeMap;

// Third Party
use tokio::sync::mpsc::Sender;
use tonic::transport::Channel;
use tonic::Status;

// Local
use super::types::{DealingValue, ProtocolRoundIndex, PublicKey};
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
