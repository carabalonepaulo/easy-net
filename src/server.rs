use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use enet_sys::{
    _ENetPeer, ENET_HOST_ANY, ENetAddress, ENetEvent, ENetPeer, enet_deinitialize,
    enet_host_create, enet_host_destroy, enet_host_flush, enet_host_service, enet_packet_create,
    enet_packet_destroy, enet_peer_disconnect, enet_peer_send,
};
use gen_slab::GenSlab;

pub const EVENT_CONNECT: i32 = 1;
pub const EVENT_DISCONNECT: i32 = 2;
pub const EVENT_RECEIVE: i32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to initialize")]
    FailedToInit,
    #[error("failed to create host")]
    FailedToCreateHost,
}

#[derive(Debug)]
pub enum ServerEvent {
    ClientConnect(u64),
    ClientDisconnected(u64),
    PacketReceived(u64, Vec<u8>),
}

pub struct Server {
    host: *mut enet_sys::ENetHost,
    events: VecDeque<ServerEvent>,
    clients: GenSlab<*mut _ENetPeer>,
}

unsafe impl Send for Server {}

impl Server {
    pub fn new(port: u16, max_conn: usize) -> Result<Self, Error> {
        unsafe {
            if enet_sys::enet_initialize() != 0 {
                return Err(Error::FailedToInit);
            }

            let addr = ENetAddress {
                host: ENET_HOST_ANY,
                port,
            };

            let host = enet_host_create(&addr, max_conn, 1, 0, 0);
            if host.is_null() {
                return Err(Error::FailedToCreateHost);
            }

            Ok(Self {
                host,
                events: VecDeque::new(),
                clients: GenSlab::with_capacity(max_conn),
            })
        }
    }

    pub fn poll(&mut self, timeout: Duration) -> usize {
        unsafe {
            let mut ev: ENetEvent = std::mem::zeroed();
            let mut count = 0;
            let start = Instant::now();
            let mut remaining = timeout;

            loop {
                if enet_host_service(self.host, &mut ev, remaining.as_millis() as u32) > 0 {
                    match ev.type_ {
                        EVENT_CONNECT => {
                            let id = self.clients.insert(ev.peer as *mut _);
                            (*ev.peer).data = id as *mut _;
                            self.events.push_back(ServerEvent::ClientConnect(id));
                            count += 1;
                        }
                        EVENT_DISCONNECT => {
                            let id = (*ev.peer).data as u64;
                            self.clients.remove(id);
                            self.events.push_back(ServerEvent::ClientDisconnected(id));
                            count += 1;
                        }
                        EVENT_RECEIVE => {
                            let id = (*ev.peer).data as u64;

                            let data = std::slice::from_raw_parts(
                                (*ev.packet).data,
                                (*ev.packet).dataLength,
                            );
                            self.events
                                .push_back(ServerEvent::PacketReceived(id, data.to_vec()));
                            enet_packet_destroy(ev.packet);
                            count += 1;
                        }
                        _ => {}
                    }
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

    pub fn send_to(&mut self, id: u64, buf: &[u8]) {
        let Some(peer_ptr) = self.clients.get(id).copied() else {
            return;
        };
        unsafe {
            let peer = peer_ptr as *mut ENetPeer;
            let packet = enet_packet_create(buf.as_ptr() as _, buf.len(), 1);
            enet_peer_send(peer, 0, packet);
        }
    }

    pub fn send_to_many<F>(&mut self, buf: &[u8], mut filter: F)
    where
        F: FnMut(u64) -> bool,
    {
        unsafe {
            let mut sent = false;
            let packet = enet_packet_create(buf.as_ptr() as _, buf.len(), 1);
            for (id, peer_ptr) in self.clients.iter() {
                let peer = *peer_ptr as *mut ENetPeer;
                if filter(id) {
                    sent = true;
                    enet_peer_send(peer, 0, packet);
                }
            }
            if !sent {
                enet_packet_destroy(packet);
            }
        }
    }

    pub fn kick(&mut self, id: u64) {
        if let Some(peer_ptr) = self.clients.get(id).copied() {
            unsafe {
                let peer = peer_ptr as *mut ENetPeer;
                enet_peer_disconnect(peer, 0);
            }
        }
    }

    pub fn kick_many<F>(&mut self, mut filter: F)
    where
        F: FnMut(u64) -> bool,
    {
        unsafe {
            for (id, peer_ptr) in self.clients.iter() {
                if filter(id) {
                    let peer = *peer_ptr as *mut ENetPeer;
                    enet_peer_disconnect(peer, 0);
                }
            }
        }
    }

    pub fn next_event(&mut self) -> Option<ServerEvent> {
        self.events.pop_front()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        unsafe {
            enet_host_flush(self.host);
            enet_host_destroy(self.host);
            enet_deinitialize();
        }
    }
}
