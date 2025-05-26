//! actor system
//! - choice 1: implement actor system using tokio::sync::mpsc ourself 
//!             below is a premature implementation
//! - choice 2: use https://github.com/tqwewe/kameo (for now)

use std::collections::{HashMap, HashSet};

use tokio::sync::mpsc::{Receiver, Sender};

use super::message::Message;


/// our target actor system take care of 
/// - (layer 1) communicate messages between local / remote nodes 
/// 
/// - (layer 2) maintain network topology between nodes
/// 
pub struct Actor {
    pub name: String,
    pub rcv: Receiver<Message>,
    pub snd_to_mgr: Sender<Message>,
    pub snds_to_sub: HashMap<String, Sender<Message>>,
}

impl Actor {
    pub fn new(
        name: String, 
        rcv: Receiver<Message>, 
        snd_to_mgr: Sender<Message>
    ) -> Self {
        Self {
            name,
            rcv,
            snd_to_mgr,
            snds_to_sub: HashMap::new(),
        }
    }

    pub async fn send_to_mgr(&self, msg: Message) {
        let _ = self.snd_to_mgr.send(msg).await;
    }

    pub async fn send_to_subs(&self, msg: Message) {
        for snd in self.snds_to_sub.values() {
            let _ = snd.send(msg.clone()).await;
        }
    }

    pub async fn receive(&mut self) -> Option<Message> {
        self.rcv.recv().await
    }

    pub async fn handle_subscribe(&mut self, msg: Message, respond_msg: Message) {
        match msg {
            Message::Subscribe { 
                subscriber, 
                sender_to_subscriber 
            } => {
                self.snds_to_sub.insert(subscriber, sender_to_subscriber.clone());
                let _ = sender_to_subscriber.send(respond_msg).await.unwrap();
            },
            _ => panic!(),
        }
    }


}