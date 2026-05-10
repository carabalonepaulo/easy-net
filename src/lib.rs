mod client;
mod server;

use std::sync::OnceLock;

pub use client::*;
pub use server::*;

pub const EVENT_CONNECT: i32 = 1;
pub const EVENT_DISCONNECT: i32 = 2;
pub const EVENT_RECEIVE: i32 = 3;

static ENET: OnceLock<()> = OnceLock::new();

fn ensure_enet_init() {
    ENET.get_or_init(|| unsafe {
        if enet_sys::enet_initialize() != 0 {
            panic!("failed to init enet");
        }
    });
}
