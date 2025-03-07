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

use crate::net::{ArpHdr, EthHdr, Ip4Hdr, PacketBufMut, UdpHdr};

#[derive(Default)]
pub struct Packet<'a> {
    pub packet_buf: PacketBufMut<'a>,
    pub eth_hdr:    Option<&'a mut EthHdr>,
    pub arp_hdr:    Option<&'a mut ArpHdr>,
    pub ip4_hdr:    Option<&'a mut Ip4Hdr>,
    pub udp_hdr:    Option<&'a mut UdpHdr>,
    pub l4_payload: Option<&'a mut [u8]>,
}

impl Packet<'_> {
    pub fn new(pkt: *mut u8, len: usize) -> Self {
        Packet::<'_> {
            packet_buf: PacketBufMut::from_raw_parts(pkt, len),
            ..Default::default()
        }
    }
}
