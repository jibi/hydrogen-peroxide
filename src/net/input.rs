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

use std::convert::TryInto;

use crate::{
    net,
    net::{ArpHdr, EthHdr, EthType, Ip4Hdr, IpProto, Packet, Result, UdpHdr},
    xsk,
};

impl net::Net {
    pub fn do_rx_packet(&mut self, desc: xsk::Desc) -> Result<()> {
        let mut packet = Packet::new(desc.packet(), desc.len());

        let eth_hdr = EthHdr::from_packet_buf(&mut packet.packet_buf)?;
        let eth_addr = eth_hdr.eth_address;

        packet.eth_hdr = Some(eth_hdr);

        match net::utils::ntohs(eth_addr).try_into() {
            Ok(EthType::IP4) => self.rx_ip4_packet(&mut packet)?,
            Ok(EthType::ARP) => self.rx_arp_packet(&mut packet)?,
            Err(_) => return Ok(()),
        }

        Ok(())
    }

    fn rx_ip4_packet<'a>(&mut self, packet: &'a mut Packet<'a>) -> Result<()> {
        let ip4 = Ip4Hdr::from_packet_buf(&mut packet.packet_buf)?;
        if ip4.proto != IpProto::UDP as u8 {
            return Ok(());
        }
        packet.ip4_hdr = Some(ip4);

        self.update_arp_cache_from_ip(packet);

        let udp = UdpHdr::from_packet_buf(&mut packet.packet_buf)?;
        let len = net::utils::ntohs(udp.len);

        packet.udp_hdr = Some(udp);

        let l4_payload = packet
            .packet_buf
            .get_bytes_mut(len as usize - std::mem::size_of::<UdpHdr>())?;

        packet.l4_payload = Some(l4_payload);

        let source_address =
            std::net::Ipv4Addr::from(net::utils::ntohl(packet.ip4_hdr.as_ref().unwrap().src_addr));

        let source_port = net::utils::ntohs(packet.udp_hdr.as_ref().unwrap().src_port);

        let socket = net::app::Socket {
            source_address,
            source_port,
        };

        self.app.rx_payload(
            &mut *self.netstack.write().unwrap(),
            &socket,
            packet.l4_payload.as_mut().unwrap(),
        )?;

        Ok(())
    }

    fn rx_arp_packet(&mut self, packet: &mut Packet) -> Result<()> {
        let arp = ArpHdr::from_packet_buf(&mut packet.packet_buf)?;
        packet.arp_hdr = Some(arp);

        self.update_arp_cache_from_arp(packet);

        self.send_arp_reply(packet)?;

        Ok(())
    }

    fn update_arp_cache_from_ip(&mut self, packet: &mut Packet) {
        let mut netstack = self.netstack.write().unwrap();

        let mac = packet.eth_hdr.as_ref().unwrap().src_address;
        let ip = packet.ip4_hdr.as_ref().unwrap().src_addr;

        netstack
            .arp_table
            .insert(std::net::Ipv4Addr::from(net::utils::ntohl(ip)), mac);
    }

    fn update_arp_cache_from_arp(&mut self, packet: &mut Packet) {
        let mut netstack = self.netstack.write().unwrap();

        let mac = packet.eth_hdr.as_ref().unwrap().src_address;
        let ip = packet.arp_hdr.as_ref().unwrap().sender_proto_addr;

        netstack
            .arp_table
            .insert(std::net::Ipv4Addr::from(u32::from_be_bytes(ip)), mac);
    }
}
