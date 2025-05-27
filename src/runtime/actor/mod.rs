//! actor system
//! - choice 1: implement actor system using tokio::sync::mpsc ourself 
//!             below is a premature implementation
//! - choice 2: use https://github.com/tqwewe/kameo (for now)

use std::collections::{HashMap, HashSet};

use tokio::sync::mpsc::{Receiver, Sender};

use super::message::Message;



/// - (layer 1: actor system) communicate messages between local / remote nodes 
///    use kameo
/// - (layer 2: pub/subscribers) maintain network topology between nodes
///    our customized 
pub struct Actor {
    pub name: String,
    pub rcv: Receiver<Message>,
    pub snd_to_mgr: Sender<Message>,
    pub snds_to_sub: HashMap<String, Sender<Message>>,
}

