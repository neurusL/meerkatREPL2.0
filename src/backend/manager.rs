use std::collections::HashMap;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{backend::message::Message, frontend::typecheck::Type};

pub struct Manager {
    pub senders_to_workers: HashMap<String, Sender<Message>>,
    pub typing_env: HashMap<String, Type>,
}
