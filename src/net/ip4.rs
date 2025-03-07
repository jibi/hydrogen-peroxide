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

use std::{fmt, mem, net::Ipv4Addr, slice};

use crate::{
    net,
    net::{PacketBufMut, Result},
};

#[repr(C)]
pub struct Ip4Hdr {
    pub hdr_len_version:   u8,
    pub tos:               u8,
    pub total_len:         u16,
    pub id:                u16,
    pub flags_frag_offset: u16,
    pub ttl:               u8,
    pub proto:             u8,
    pub checksum:          u16,
    pub src_addr:          u32,
    pub dst_addr:          u32,
}

pub enum IpProto {
    UDP = 17,
}

const IP4_VERSION: u8 = 4;

pub enum IpFlags {
    Reserved = 1,
    DontFragment = 2,
    MoreFragment = 4,
}

impl Ip4Hdr {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn from_packet_buf<'a>(packet: &mut PacketBufMut<'a>) -> Result<&'a mut Self> {
        packet
            .get_bytes_mut(mem::size_of::<Self>())
            .map(|l3_slice| unsafe { &mut *(l3_slice.as_mut_ptr() as *mut Ip4Hdr) })
    }

    pub fn with_packet_buf<'a>(packet: &mut PacketBufMut<'a>) -> Result<&'a mut Self> {
        let hdr = Self::from_packet_buf(packet)?;

        hdr.set_version(IP4_VERSION);
        hdr.set_hdr_len(5);
        hdr.tos = 0;
        hdr.id = 0;

        hdr.set_flags(IpFlags::DontFragment as u8);
        hdr.set_frag_offset(0);
        hdr.ttl = 64;

        Ok(hdr)
    }

    pub fn hdr_len(&self) -> u8 {
        self.hdr_len_version & 0xf
    }

    pub fn version(&self) -> u8 {
        (self.hdr_len_version & 0xf0) >> 4
    }

    pub fn flags(&self) -> u8 {
        (net::utils::ntohs(self.flags_frag_offset) >> 13) as u8
    }

    pub fn frag_offset(&self) -> u16 {
        net::utils::ntohs(self.flags_frag_offset) & ((1 << 13) - 1)
    }

    pub fn set_hdr_len(&mut self, v: u8) -> &mut Self {
        self.hdr_len_version = (self.hdr_len_version & 0xf0) | (v & 0xf);
        self
    }

    pub fn set_version(&mut self, v: u8) -> &mut Self {
        self.hdr_len_version = (self.hdr_len_version & 0xf) | ((v & 0xf) << 4);
        self
    }

    pub fn set_total_length(&mut self, v: u16) -> &mut Self {
        self.total_len = net::utils::htons(v);
        self
    }

    pub fn set_flags(&mut self, v: u8) -> &mut Self {
        let mut flags_frag_offset = net::utils::ntohs(self.flags_frag_offset);
        flags_frag_offset = (flags_frag_offset & 0x2000) | ((v as u16 & 0x7) << 13);
        self.flags_frag_offset = net::utils::htons(flags_frag_offset);
        self
    }

    pub fn set_frag_offset(&mut self, v: u16) -> &mut Self {
        let mut flags_frag_offset = net::utils::ntohs(self.flags_frag_offset);
        flags_frag_offset = (flags_frag_offset & 0xe000) | (v & 0x2000);
        self.flags_frag_offset = net::utils::htons(flags_frag_offset);
        self
    }

    pub fn udp(&mut self) -> &mut Self {
        self.proto = IpProto::UDP as u8;
        self
    }

    pub fn set_src_address(&mut self, v: Ipv4Addr) -> &mut Self {
        self.src_addr = net::utils::htonl(v.into());
        self
    }

    pub fn set_dst_address(&mut self, v: Ipv4Addr) -> &mut Self {
        self.dst_addr = net::utils::htonl(v.into());
        self
    }

    pub fn calc_checksum(&mut self) {
        self.checksum = 0;

        self.checksum = unsafe {
            net::utils::ntohs({
                let buf: *const u8 = &*self as *const net::ip4::Ip4Hdr as *const u8;
                let slice = slice::from_raw_parts(buf, 20);

                let mut sum: u32 = 0;
                let len = 20;
                let mut i = 0;

                while i < len {
                    let word = (slice[i] as u32) << 8 | (slice[i + 1] as u32);

                    sum += word;
                    i += 2;
                }

                while sum >> 16 != 0 {
                    sum = (sum >> 16) + (sum & 0xffff);
                }

                !sum as u16
            })
        };
    }
}

impl fmt::Debug for Ip4Hdr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  Ip4Hdr {{ hdr_len: {}, version: {}, tos: {}, total_len: {}, \
            id: 0x{:04x}, flags: 0x{:x}, frag_offset: 0x{:x}, \
            ttl: {}, proto: {}, checksum: 0x{:02x}, \
            src_addr: {}, dst_addr: {} }}",
            self.hdr_len(),
            self.version(),
            self.tos,
            net::utils::ntohs(self.total_len),
            net::utils::ntohs(self.id),
            self.flags(),
            net::utils::ntohs(self.frag_offset()),
            self.ttl,
            self.proto,
            net::utils::ntohs(self.checksum),
            Ipv4Addr::from(net::utils::ntohl(self.src_addr)),
            Ipv4Addr::from(net::utils::ntohl(self.dst_addr)),
        )
    }
}
