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

use crate::net;
use crate::net::Result;

use std::convert::TryFrom;
use std::result;
use std::{fmt, mem};

#[repr(C)]
pub struct EthHdr {
    pub dst_address: [u8; 6],
    pub src_address: [u8; 6],
    pub eth_address: u16,
}

impl EthHdr {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn from_packet_buf<'a>(packet: &mut net::PacketBufMut<'a>) -> Result<&'a mut Self> {
        packet
            .get_bytes_mut(mem::size_of::<Self>())
            .map(|l2_slice| unsafe { &mut *(l2_slice.as_mut_ptr() as *mut EthHdr) })
    }

    pub fn with_packet_buf<'a>(packet: &mut net::PacketBufMut<'a>) -> Result<&'a mut Self> {
        Self::from_packet_buf(packet)
    }

    pub fn set_src_address(&mut self, a: [u8; 6]) -> &mut Self {
        self.src_address = a;
        self
    }

    pub fn set_dst_address(&mut self, a: [u8; 6]) -> &mut Self {
        self.dst_address = a;
        self
    }

    pub fn arp(&mut self) -> &mut Self {
        self.eth_address = net::utils::htons(net::eth::EthType::ARP as u16);
        self
    }

    pub fn ip4(&mut self) -> &mut Self {
        self.eth_address = net::utils::htons(net::eth::EthType::IP4 as u16);
        self
    }
}

impl fmt::Debug for EthHdr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EthHdr {{ dst_address: {}, src_address: {}, eth_address: 0x{:04x} }}",
            net::utils::mac_to_string(self.dst_address),
            net::utils::mac_to_string(self.src_address),
            net::utils::ntohs(self.eth_address),
        )
    }
}

#[derive(Clone)]
pub enum EthType {
    IP4 = 0x0800,
    ARP = 0x0806,
}

impl TryFrom<u16> for EthType {
    type Error = ();

    fn try_from(x: u16) -> result::Result<Self, Self::Error> {
        use EthType::*;

        match x {
            x if x == IP4 as u16 => Ok(IP4),
            x if x == ARP as u16 => Ok(ARP),
            _ => Err(()),
        }
    }
}
