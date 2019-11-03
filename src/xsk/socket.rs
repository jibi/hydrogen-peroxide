// Copyright (C) 2020 Gilberto "jibi" Bertin <me@jibi.io>
//
// This file is part of hydrogen peroxyde.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnumem.org/licenses/>.

//! XSK sockets.

use std::{
    ffi::CString,
    io, mem, ptr,
    sync::{Arc, RwLock},
};

use crate::{
    xsk,
    xsk::{
        net, Configuration, ConsRing, Desc, Error::*, NeedsWakeup, ProdRing, Result, Runner, Umem,
    },
};

/// An XSK soscket.
pub struct Socket {
    socket:    *mut xsk::sys::xsk_socket,
    rx_socket: Option<RxSocket>,
    tx_socket: Option<TxSocket>,
}

unsafe impl Send for Socket {}

impl Socket {
    /// Create a new XSK socket.
    pub fn new(
        cfg: Arc<Configuration>,
        umem: &mut Arc<RwLock<Umem>>,
        queue: usize,
        pipe_reader_fd: i32,
    ) -> Result<Self> {
        let libbpf_flags = xsk::sys::XSK_LIBBPF_FLAGS__INHIBIT_PROG_LOAD;
        let xdp_flags = xsk::sys::XDP_FLAGS_UPDATE_IF_NOEXIST | cfg.mode().into_xdp_flags();
        let bind_flags = cfg.needs_wakeup().into_bind_flags() | cfg.mode().into_bind_flags();

        // Initialize the XSK socket.
        let xsk_cfg = xsk::sys::xsk_socket_config {
            rx_size: cfg.rx_size() as u32,
            tx_size: cfg.tx_size() as u32,
            libbpf_flags,
            xdp_flags,
            bind_flags,
        };

        let (socket, tx, rx) = {
            let umem = umem.write().unwrap();

            let mut socket: *mut xsk::sys::xsk_socket = ptr::null_mut();
            let mut rx_ring: xsk::sys::xsk_ring_cons = unsafe { mem::zeroed() };
            let mut tx_ring: xsk::sys::xsk_ring_prod = unsafe { mem::zeroed() };

            let interface_cstr = CString::new(String::from(cfg.interface())).unwrap();

            let ret = unsafe {
                xsk::sys::xsk_socket__create(
                    &mut socket,
                    interface_cstr.as_ptr(),
                    queue as u32,
                    umem.umem,
                    &mut rx_ring,
                    &mut tx_ring,
                    &xsk_cfg,
                )
            };

            if ret != 0 {
                return Err(XskSocketCreateFailed(-ret));
            }

            // Initialize the RX ring.
            let rx = ConsRing::new_from_xsk_ring_cons(
                umem.frame_allocator.clone(),
                rx_ring,
                cfg.rx_size(),
            );

            // Initialize and populate the TX ring.
            let mut tx = ProdRing::new_from_xsk_ring_prod(
                umem.frame_allocator.clone(),
                tx_ring,
                cfg.tx_size(),
            );
            {
                let mut frame_allocator = umem.frame_allocator.write().unwrap();

                for i in 0..cfg.tx_size() {
                    let addr = frame_allocator.alloc_frame().unwrap();
                    tx.fill_addr(i as u32, addr);
                }
            }

            (socket, tx, rx)
        };

        let poll_fds: [libc::pollfd; 2] = [
            libc::pollfd {
                fd:      unsafe { xsk::sys::xsk_socket__fd(socket) },
                events:  libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd:      pipe_reader_fd,
                events:  libc::POLLIN,
                revents: 0,
            },
        ];

        let ready_for_tx_slots = vec![false; cfg.tx_size()];
        let current_tx_slot = 0;

        Ok(Socket {
            socket,

            rx_socket: Some(RxSocket {
                rx,
                umem: umem.clone(),
                poll_fds,
            }),

            tx_socket: Some(TxSocket {
                tx,
                socket,
                umem: umem.clone(),
                needs_wakeup: cfg.needs_wakeup(),
                ready_for_tx_slots,
                current_tx_slot,
                configuration: cfg,
            }),
        })
    }

    /// Returns the fd associated with the XSK socket.
    pub fn fd(&self) -> i32 {
        unsafe { xsk::sys::xsk_socket__fd(self.socket) }
    }

    /// Returns an owned `RxSocket` socket.
    pub fn take_rx_socket(&mut self) -> RxSocket {
        self.rx_socket.take().unwrap()
    }

    /// Returns an owned `TxSocket` socket.
    pub fn take_tx_socket(&mut self) -> TxSocket {
        self.tx_socket.take().unwrap()
    }
}

/// An object responsible for handling the RX logic of an XSK [`Socket`].
pub struct RxSocket {
    rx:   ConsRing,
    umem: Arc<RwLock<Umem>>,

    poll_fds: [libc::pollfd; 2],
}

impl RxSocket {
    /// Start the RX loop
    pub fn rx_loop(runner: Runner, mut net: Box<dyn net::Net>, mut socket: RxSocket) {
        let umem = socket.umem.clone();
        while runner.is_running() {
            socket
                .run_rx_loop(&umem, &mut net)
                .unwrap_or_else(|e| eprintln!("Error in receive loop: {}", e));
        }
    }

    /// poll() the fd associated with the [`RxSocket`].
    pub fn poll(&mut self) -> Result<i32> {
        let nfds = self.poll_fds.len() as u64;
        let timeout = -1 as libc::c_int;

        let ret = unsafe { libc::poll(self.poll_fds.as_mut_ptr(), nfds, timeout) };
        if ret == -1 {
            let errno = io::Error::last_os_error().raw_os_error().unwrap();
            if errno != libc::EINTR {
                return Err(XskSocketPollFailed(errno));
            }
        }

        Ok(ret)
    }

    /// RX loop
    fn run_rx_loop(&mut self, umem: &Arc<RwLock<Umem>>, net: &mut Box<dyn net::Net>) -> Result<()> {
        match self.poll() {
            Ok(ret) => {
                if ret <= 0 {
                    return Ok(());
                }
            }
            Err(e) => return Err(e),
        };

        let mut idx_rx = 0;
        let rcvd = self.rx.peek(xsk::BATCH_SIZE, &mut idx_rx);

        {
            let mut umem = umem.write().unwrap();
            umem.reclaim_fq_bufs(self, rcvd)
                .unwrap_or_else(|e| eprintln!("Error reclaiming FQ buffers: {}", e));
        }

        if rcvd == 0 {
            return Ok(());
        }

        for _ in 0..rcvd {
            let desc = self.rx.get_desc(idx_rx);

            net.rx_packet(desc)
                .unwrap_or_else(|e| eprintln!("Error receiving packet: {}", e));
            idx_rx += 1;
        }

        self.rx.release(rcvd);

        Ok(())
    }
}

/// An object responsible for handling the TX logic of an XSK [`Socket`].
pub struct TxSocket {
    tx:     ProdRing,
    socket: *mut xsk::sys::xsk_socket,
    umem:   Arc<RwLock<Umem>>,

    needs_wakeup: NeedsWakeup,

    current_tx_slot:    usize,
    ready_for_tx_slots: Vec<bool>,

    // Keep a reference to the XSK configuration as it will be exposed by the Handle trait
    configuration: Arc<Configuration>,
}

impl TxSocket {
    /// Returns the XSK [`Configuration`] associated with the socket.
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    /// Returns the next TX descriptor available in the socket's TX ring.
    pub fn next_tx_slot(&mut self) -> Result<Desc> {
        let mut tx_idx = 0;
        if self.tx.reserve(1, &mut tx_idx) != 1 {
            return Err(XskTxRingProdReserveFailed);
        }

        let desc = self.tx.get_desc(tx_idx);

        Ok(desc)
    }

    /// Mark the `desc` [`Desc`] as ready to be transmitted and transmits all
    /// consecutive ready-to-be-transmitted descriptors from the beginning of the ring.
    pub fn tx(&mut self, desc: &Desc) -> Result<()> {
        self.mark_slot_ready_for_tx(desc.index());

        let ready_for_tx_slots_count = self.ready_for_tx_slot_counts();
        if ready_for_tx_slots_count == 0 {
            return Ok(());
        }

        self.tx.submit(ready_for_tx_slots_count);

        if self.needs_wakeup.value {
            if self.tx.needs_wakeup() {
                self.sendto()?;
            }
        } else {
            self.sendto()?;
        }

        self.umem
            .write()
            .unwrap()
            .reclaim_cq_bufs(ready_for_tx_slots_count);

        Ok(())
    }

    /// Mark slot at index `index` as ready to be transmitted.
    fn mark_slot_ready_for_tx(&mut self, index: usize) {
        self.ready_for_tx_slots[index] = true;
    }

    /// Returns the number of consecutive slots (from the current one) in the TX ring which are
    /// ready to be transmitted.
    fn ready_for_tx_slot_counts(&mut self) -> usize {
        let slots_count = self.ready_for_tx_slots.len();
        let mut ready_for_tx_slots_count = 0;

        for _ in 0..slots_count {
            if !self.ready_for_tx_slots[self.current_tx_slot] {
                break;
            }

            self.ready_for_tx_slots[self.current_tx_slot] = false;
            self.current_tx_slot = (self.current_tx_slot + 1) % slots_count;
            ready_for_tx_slots_count += 1;
        }

        ready_for_tx_slots_count
    }

    /// Returns the fd associated with the TxSocket.
    fn fd(&self) -> i32 {
        unsafe { xsk::sys::xsk_socket__fd(self.socket) }
    }

    /// Calls `sendto()` on the fd associated with the TxSocket to transmit all consecutive
    /// ready-to-be-transmitted descriptors from the beginning of the ring.
    fn sendto(&self) -> Result<()> {
        let ret = unsafe {
            libc::sendto(
                self.fd(),
                ptr::null(),
                0,
                libc::MSG_DONTWAIT,
                ptr::null(),
                0,
            )
        };

        if ret == -1 {
            let errno = io::Error::last_os_error().raw_os_error().unwrap();
            if !(errno == libc::ENOBUFS
                || errno == libc::EAGAIN
                || errno == libc::EBUSY
                || errno == libc::ENETDOWN)
            {
                return Err(XskTxSendtoFailed(errno));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TUN_IFNAME: &str = "hype_tun0";

    pub fn init_tun(ifname: String) -> tun::platform::Device {
        let mut config = tun::Configuration::default();
        config.name(ifname.clone()).layer(tun::Layer::L2).up();

        tun::create(&config).unwrap()
    }

    #[test]
    fn test_new() {
        let _dev = init_tun(TUN_IFNAME.to_string());

        let mut cfg = Configuration::default();
        cfg.set_interface(TUN_IFNAME.to_string());
        cfg.set_needs_wakeup(NeedsWakeup::new(false));

        let cfg = Arc::new(cfg);

        let mut umem = Arc::new(RwLock::new(Umem::new(&cfg).unwrap()));

        let mut pipe_fds = [0; 2];
        unsafe {
            libc::pipe(pipe_fds.as_mut_ptr());
        }

        let socket = Socket::new(cfg, &mut umem, 0, pipe_fds[0]);

        assert!(socket.is_ok());
    }
}
