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

use std::mem;

use crate::{
    net,
    net::{app::Socket, ArpHdr, EthHdr, Ip4Hdr, Packet, PacketBufMut, UdpHdr},
};

impl net::Net {
    pub fn send_arp_reply(&mut self, rx_packet: &Packet<'_>) -> anyhow::Result<()> {
        let netstack = self.netstack.write().unwrap();
        let (mut tx_desc, mut packet_buf) = {
            let mut xsk_handle = netstack.xsk_handle.write().unwrap();

            let tx_desc = xsk_handle.next_tx_slot()?;
            let packet_buf = PacketBufMut::from_raw_parts(
                tx_desc.packet(),
                xsk_handle.configuration().frame_size(),
            );

            (tx_desc, packet_buf)
        };

        let rx_eth = rx_packet.eth_hdr.as_ref().unwrap();
        EthHdr::from_packet_buf(&mut packet_buf)?
            .set_src_address(netstack.iface_mac)
            .set_dst_address(rx_eth.src_address)
            .arp();

        let rx_arp = rx_packet.arp_hdr.as_ref().unwrap();
        ArpHdr::from_packet_buf(&mut packet_buf)?
            .arp_reply_ip()
            .set_sender_hw_address(netstack.iface_mac)
            .set_sender_proto_address(rx_arp.target_proto_addr)
            .set_target_hw_address(rx_arp.sender_hw_addr)
            .set_target_proto_address(rx_arp.sender_proto_addr);

        tx_desc.set_len(packet_buf.as_slice().len());
        netstack.xsk_handle.write().unwrap().tx(&tx_desc)?;

        Ok(())
    }
}

impl net::app::Handle for net::NetStack {
    /// Return a new `net::app::PayloadBuf` object.
    fn new_tx_payload_buf<'a>(&mut self) -> anyhow::Result<net::app::PayloadBuf<'a>> {
        let mut xsk_handle = self.xsk_handle.write().unwrap();

        // Get a new TX descriptor from XSK
        let xdp_desc = xsk_handle.next_tx_slot()?;

        // Wrap the TX descriptor in a `PacketBufMut` object
        let mut packet_buf = PacketBufMut::from_raw_parts(
            xdp_desc.packet(),
            xsk_handle.configuration().frame_size(),
        );

        // Seek packet_buf to the offset of the L4 payload, so that the app will be able to write
        // the payload data to the correct offset.
        packet_buf
            .seek(mem::size_of::<EthHdr>() + mem::size_of::<Ip4Hdr>() + mem::size_of::<UdpHdr>())?;

        Ok(net::app::PayloadBuf::new(xdp_desc, packet_buf))
    }

    fn send_payload(
        &mut self,
        socket: &Socket,
        payload_buf: &mut net::app::PayloadBuf,
    ) -> anyhow::Result<()> {
        // Set the position of the packet buffer back to 0 so that we can write the L2, L3 and L4 headers
        payload_buf.packet_buf().seek(0)?;
        let packet_len = payload_buf.packet_buf().as_slice().len();

        EthHdr::with_packet_buf(payload_buf.packet_buf())?
            .set_src_address(self.iface_mac)
            // TODO: handle missing ARP entry
            .set_dst_address(*self.arp_table.get(&socket.source_address).unwrap())
            .ip4();

        Ip4Hdr::with_packet_buf(payload_buf.packet_buf())?
            .set_total_length((packet_len - std::mem::size_of::<EthHdr>()) as u16)
            .udp()
            .set_src_address(self.bind_address)
            .set_dst_address(socket.source_address)
            .calc_checksum();

        UdpHdr::with_packet_buf(payload_buf.packet_buf())?
            .set_src_port(self.bind_port)
            .set_dst_port(socket.source_port)
            .set_length((packet_len - mem::size_of::<EthHdr>() - mem::size_of::<Ip4Hdr>()) as u16);

        payload_buf.xdp_desc().set_len(packet_len);

        self.xsk_handle
            .write()
            .unwrap()
            .tx(payload_buf.xdp_desc())?;

        Ok(())
    }
}
