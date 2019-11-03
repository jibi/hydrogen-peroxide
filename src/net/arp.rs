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
use crate::net::{EthType, PacketBufMut, Result};

use std::net::Ipv4Addr;
use std::{fmt, mem};

pub enum Htype {
    Ethernet = 0x1,
}

pub enum ArpOpcode {
    REQUEST = 0x1,
    REPLY = 0x2,
}

#[repr(C)]
pub struct ArpHdr {
    pub hw_type:           u16,
    pub proto_type:        u16,
    pub hw_addr_len:       u8,
    pub proto_addr_len:    u8,
    pub opcode:            u16,
    //TODO: use a slice to make it work with v6
    pub sender_hw_addr:    [u8; 6],
    pub sender_proto_addr: [u8; 4],
    pub target_hw_addr:    [u8; 6],
    pub target_proto_addr: [u8; 4],
}

impl ArpHdr {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn from_packet_buf<'a>(packet: &mut PacketBufMut<'a>) -> Result<&'a mut Self> {
        packet
            .get_bytes_mut(mem::size_of::<Self>())
            .map(|l3_slice| unsafe { &mut *(l3_slice.as_mut_ptr() as *mut ArpHdr) })
    }

    pub fn arp_reply_ip(&mut self) -> &mut Self {
        self.hw_type = net::utils::htons(Htype::Ethernet as u16);
        self.proto_type = net::utils::htons(EthType::IP4 as u16);
        self.hw_addr_len = 6;
        self.proto_addr_len = 4;
        self.opcode = net::utils::ntohs(ArpOpcode::REPLY as u16);

        self
    }

    pub fn set_sender_hw_address(&mut self, v: [u8; 6]) -> &mut Self {
        self.sender_hw_addr = v;
        self
    }

    pub fn set_sender_proto_address(&mut self, v: [u8; 4]) -> &mut Self {
        self.sender_proto_addr = v;
        self
    }

    pub fn set_target_hw_address(&mut self, v: [u8; 6]) -> &mut Self {
        self.target_hw_addr = v;
        self
    }

    pub fn set_target_proto_address(&mut self, v: [u8; 4]) -> &mut Self {
        self.target_proto_addr = v;
        self
    }
}

impl fmt::Debug for ArpHdr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  ArpHdr {{ hw_type: {}, proto_type: {}, \
            hw_addr_len: {}, proto_addr_len: {}, opcode: {}, \
            sender_hw_addr: {}, sender_proto_addr: {}, \
            target_hw_addr: {}, target_proto_addr: {} }}",
            self.hw_type,
            self.proto_type,
            self.hw_addr_len,
            self.proto_addr_len,
            self.opcode,
            net::utils::mac_to_string(self.sender_hw_addr),
            Ipv4Addr::from(net::utils::ntohl(u32::from_be_bytes(
                self.sender_proto_addr
            ))),
            net::utils::mac_to_string(self.target_hw_addr),
            Ipv4Addr::from(net::utils::ntohl(u32::from_be_bytes(
                self.target_proto_addr
            ))),
        )
    }
}
