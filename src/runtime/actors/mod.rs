//! actor system and pub sub system
//! - choice 1: implement actor system using tokio::sync::mpsc ourself 
//!             below is a premature implementation
//! - choice 2: use https://github.com/tqwewe/kameo (for now)


/// - (layer 1: actor system) communicate messages between local / remote nodes 
///    use kameo
/// - (layer 2: pub/subscribers) maintain network topology between nodes
///    use kameo/actors/src/pubsub.rs
pub struct Actor {}
