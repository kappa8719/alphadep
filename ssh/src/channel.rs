use std::sync::{Arc, Mutex};

type RawChannel = russh::Channel<russh::client::Msg>;

#[derive(Clone)]
pub struct Channel {
    raw: Arc<Mutex<RawChannel>>,
}