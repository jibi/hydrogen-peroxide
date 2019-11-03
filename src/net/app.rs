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
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Interfaces for glueing together `net` and an app.

use std::{
    net::Ipv4Addr,
    sync::{Arc, RwLock},
};

use crate::{net::PacketBufMut, xsk};

/// Signature of the closure that `xsk` expects to call whenever it needs to allocate a new network
/// stack object.
///
/// The returned object must implement the [`crate::net::app::App`] trait.
pub type AppAllocator = dyn Fn(Arc<RwLock<dyn Handle>>) -> Box<dyn App>;

/// Trait that a generic app object must implement in order to receive payloads from the
/// `net` module.
pub trait App: Send {
    fn rx_payload(
        &mut self,
        netstack_handle: &mut dyn Handle,
        socket: &Socket,
        rx_payload: &mut [u8],
    ) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct Socket {
    pub source_address: Ipv4Addr,
    pub source_port:    u16,
}

pub struct PayloadBuf<'a> {
    xdp_desc:   xsk::Desc,
    packet_buf: PacketBufMut<'a>,
}

impl<'a> PayloadBuf<'a> {
    pub fn new(xdp_desc: xsk::Desc, packet_buf: PacketBufMut<'a>) -> Self {
        PayloadBuf {
            xdp_desc,
            packet_buf,
        }
    }

    pub fn packet_buf(&mut self) -> &mut PacketBufMut<'a> {
        &mut self.packet_buf
    }

    pub fn xdp_desc(&mut self) -> &mut xsk::Desc {
        &mut self.xdp_desc
    }
}

pub trait Handle: Sync + Send {
    fn new_tx_payload_buf<'a>(&mut self) -> anyhow::Result<PayloadBuf<'a>>;
    fn send_payload(&mut self, socket: &Socket, payload_buf: &mut PayloadBuf)
        -> anyhow::Result<()>;
}
