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

use std::{fmt, mem};

use crate::{
    net,
    net::{PacketBufMut, Result},
};

#[repr(C)]
pub struct UdpHdr {
    pub src_port: u16,
    pub dst_port: u16,
    pub len:      u16,
    pub sum:      u16,
}

impl UdpHdr {
    #[allow(clippy::cast_ptr_alignment)]
    pub fn from_packet_buf<'a>(packet: &mut PacketBufMut<'a>) -> Result<&'a mut Self> {
        packet
            .get_bytes_mut(mem::size_of::<Self>())
            .map(|l4_slice| unsafe { &mut *(l4_slice.as_mut_ptr() as *mut UdpHdr) })
    }

    pub fn with_packet_buf<'a>(packet: &mut PacketBufMut<'a>) -> Result<&'a mut Self> {
        let hdr = Self::from_packet_buf(packet)?;
        hdr.sum = 0;

        Ok(hdr)
    }

    pub fn set_src_port(&mut self, v: u16) -> &mut Self {
        self.src_port = net::utils::htons(v);
        self
    }

    pub fn set_dst_port(&mut self, v: u16) -> &mut Self {
        self.dst_port = net::utils::htons(v);
        self
    }

    pub fn set_length(&mut self, v: u16) -> &mut Self {
        self.len = net::utils::htons(v);
        self
    }
}

impl fmt::Debug for UdpHdr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "    UdpHdr {{ src_port: {}, dst_port: {}, \
            len: {}, sum: 0x{:x} }}",
            net::utils::ntohs(self.src_port),
            net::utils::ntohs(self.dst_port),
            net::utils::ntohs(self.len),
            net::utils::ntohs(self.sum),
        )
    }
}
