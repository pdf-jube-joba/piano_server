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

pub fn join() -> SendEventBinary {
    [0u8; size_of::<SendEventBinary>()]
}

pub fn disconnect() -> SendEventBinary {
    [1u8; size_of::<SendEventBinary>()]
}
