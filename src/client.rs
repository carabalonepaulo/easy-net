use std::{
    collections::VecDeque,
    ffi::CString,
    time::{Duration, Instant},
};

use enet_sys::{
    ENetAddress, ENetEvent, ENetHost, ENetPeer, enet_address_set_host, enet_host_connect,
    enet_host_create, enet_host_destroy, enet_host_flush, enet_host_service, enet_packet_create,
    enet_packet_destroy, enet_peer_disconnect, enet_peer_send,
};

use crate::{EVENT_CONNECT, EVENT_DISCONNECT, EVENT_RECEIVE, ensure_enet_init};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create host")]
    FailedToCreateHost,

    #[error("failed to resolve address")]
    FailedToResolveAddress,

    #[error("failed to connect")]
    FailedToConnect,
}

#[derive(Debug)]
pub enum ClientEvent {
    Connected,
    Disconnected,
    PacketReceived(Vec<u8>),
}

#[derive(Debug)]
pub struct Client {
    host: *mut ENetHost,
    peer: *mut ENetPeer,
    events: VecDeque<ClientEvent>,
    connected: bool,
}

unsafe impl Send for Client {}

impl Client {
    pub fn new(host: &str, port: u16) -> Result<Self, Error> {
        ensure_enet_init();

        unsafe {
            let client = enet_host_create(std::ptr::null(), 1, 1, 0, 0);
            if client.is_null() {
                return Err(Error::FailedToCreateHost);
            }

            let mut addr: ENetAddress = std::mem::zeroed();
            let host_str = CString::new(host).map_err(|_| Error::FailedToResolveAddress)?;
            if enet_address_set_host(&mut addr, host_str.as_ptr()) != 0 {
                enet_host_destroy(client);
                return Err(Error::FailedToResolveAddress);
            }

            addr.port = port;
            let peer = enet_host_connect(client, &addr, 1, 0);
            if peer.is_null() {
                enet_host_destroy(client);
                return Err(Error::FailedToConnect);
            }

            Ok(Self {
                host: client,
                peer,
                events: VecDeque::new(),
                connected: false,
            })
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn poll(&mut self, timeout: Duration) -> usize {
        unsafe {
            let mut ev: ENetEvent = std::mem::zeroed();

            let start = Instant::now();
            let mut remaining = timeout;

            let mut count = 0;

            loop {
                let ret = enet_host_service(self.host, &mut ev, remaining.as_millis() as u32);

                if ret <= 0 {
                    break;
                }

                match ev.type_ {
                    EVENT_CONNECT => {
                        self.connected = true;
                        self.events.push_back(ClientEvent::Connected);
                        count += 1;
                    }
                    EVENT_DISCONNECT => {
                        self.connected = false;
                        self.events.push_back(ClientEvent::Disconnected);
                        count += 1;
                    }
                    EVENT_RECEIVE => {
                        let data =
                            std::slice::from_raw_parts((*ev.packet).data, (*ev.packet).dataLength);
                        self.events
                            .push_back(ClientEvent::PacketReceived(data.to_vec()));
                        enet_packet_destroy(ev.packet);
                        count += 1;
                    }
                    _ => {}
                }

                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    break;
                }

                remaining = timeout - elapsed;
            }

            count
        }
    }

    pub fn next_event(&mut self) -> Option<ClientEvent> {
        self.events.pop_front()
    }

    pub fn send(&mut self, buf: &[u8]) {
        unsafe {
            let packet = enet_packet_create(buf.as_ptr() as _, buf.len(), 1);
            enet_peer_send(self.peer, 0, packet);
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe {
            enet_peer_disconnect(self.peer, 0);
            enet_host_flush(self.host);
            enet_host_destroy(self.host);
        }
    }
}
