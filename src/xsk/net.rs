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

//! Interfaces for glueing together `xsk` and a network stack.

use crate::{
    xsk,
    xsk::{Configuration, Desc},
};

/// Signature of the closure that `xsk` expects to call whenever it needs to allocate a new network
/// stack object.
///
/// The returned object must implement the [`crate::xsk::net::Net`] trait.
pub type NetAllocator = dyn Fn(Handle) -> Box<dyn Net>;

/// Trait that a generic network stack object must implement in order to receive packets from the
/// `xsk` module.
pub trait Net: Send + Sync {
    /// Callback invoked when XSK has received a new packet.
    /// `desc` is an XDP descriptor pointing to a packet buffer of a newly arrived packet.
    fn rx_packet(&mut self, desc: Desc) -> anyhow::Result<()>;
}

/// An object used to expose a minimal interface of the XSK socket to the network stack.
pub struct Handle(xsk::TxSocket);

impl Handle {
    /// Returns the XSK [`Configuration`] associated with the handle.
    pub fn configuration(&self) -> &Configuration {
        self.0.configuration()
    }

    /// Returns the next TX descriptor available in the socket's TX ring.
    pub fn next_tx_slot(&mut self) -> xsk::Result<Desc> {
        self.0.next_tx_slot()
    }

    /// Mark the `desc` [`Desc`] as ready to be transmitted and transmits all
    /// consecutive ready-to-be-transmitted descriptors from the beginning of the ring.
    pub fn tx(&mut self, desc: &Desc) -> xsk::Result<()> {
        self.0.tx(desc)
    }
}

impl From<xsk::TxSocket> for Handle {
    fn from(tx_socket: xsk::TxSocket) -> Self {
        Handle(tx_socket)
    }
}
