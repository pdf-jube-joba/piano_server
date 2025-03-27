use std::mem::size_of;

pub type ID = u32;
pub const SERVER: &str = "127.0.0.1:8000";
pub const SIZE_OF_EVENT: usize = 25;

pub type ReceiveEventBinary = [u8; SIZE_OF_EVENT];
pub fn default_r() -> ReceiveEventBinary {
    [0u8; size_of::<ReceiveEventBinary>()]
}

pub type SendEventBinary = [u8; SIZE_OF_EVENT + size_of::<ID>()];
pub fn default_s() -> SendEventBinary {
    [0u8; size_of::<SendEventBinary>()]
}

pub fn join(client_id: ID) -> SendEventBinary {
    let mut msg = [0u8; size_of::<SendEventBinary>()];
    msg[0..size_of::<ID>()].clone_from_slice(&client_id.to_le_bytes());
    msg
}

pub fn disconnect(client_id: ID) -> SendEventBinary {
    let mut msg = [u8::MAX; size_of::<SendEventBinary>()];
    msg[0..size_of::<ID>()].clone_from_slice(&client_id.to_le_bytes());
    msg
}
