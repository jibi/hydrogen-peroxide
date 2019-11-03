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

use anyhow;
use tun::Device;

use std::{
    io::{Read, Write},
    net::Ipv4Addr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

use libh2o2::{
    echo, net,
    net::{EthHdr, Ip4Hdr, PacketBufMut, UdpHdr},
    xsk,
};

static TUN_NUM: AtomicUsize = AtomicUsize::new(0);

pub fn init_tun() -> tun::platform::Device {
    let dev_name = format!("hype_tun{}", TUN_NUM.fetch_add(1, Ordering::SeqCst));

    let mut config = tun::Configuration::default();
    config
        .name(dev_name.clone())
        .layer(tun::Layer::L2)
        .queues(4);

    let mut dev = tun::create(&config).unwrap();

    // Disable IPv6 autoconf as we don't want to assign the interface an IPv6 address
    let path = format!("/proc/sys/net/ipv6/conf/{}/addr_gen_mode", dev_name);
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(b"1\n").unwrap();

    dev.enabled(true).unwrap();

    dev
}

pub fn init_xsk(
    dev: &tun::platform::Device,
    queues: Vec<usize>,
    socks_per_queue: usize,
    repeated: bool,
) -> xsk::Xsk {
    let mut xsk_cfg = xsk::Configuration::default();

    let net_allocator: Box<xsk::net::NetAllocator> =
        Box::new(move |xsk_handle: xsk::net::Handle| {
            let app_allocator: Box<net::app::AppAllocator> =
                Box::new(move |net_handle: Arc<RwLock<dyn net::app::Handle>>| {
                    Box::new(echo::EchoApp::new(net_handle, repeated))
                });

            let mut net_cfg = net::Configuration::default();

            net_cfg
                .set_app_allocator(app_allocator)
                .set_xsk_handle(xsk_handle);

            Box::new(net::Net::new(net_cfg))
        });

    xsk_cfg
        .set_interface(dev.name())
        .set_bind_address(std::net::Ipv4Addr::new(192, 18, 42, 42))
        .set_bind_port(1234)
        .set_rx_size(256)
        .set_tx_size(256)
        .set_net_allocator(net_allocator)
        .set_queues(queues)
        .set_needs_wakeup(xsk::NeedsWakeup::new(false))
        .set_mode(xsk::XskMode::Drv)
        .set_socks_per_queue(socks_per_queue);

    xsk::Xsk::new(xsk_cfg).unwrap()
}

pub fn test_echo_server(dev: &mut tun::platform::Device, queue: usize) {
    let mut tx_buf = [0 as u8; 1024];
    let mut tx_packet = build_tx_packet(dev.name(), &mut tx_buf, 8000).unwrap();

    test_echo_server_with_tx_packet(dev, queue, &mut tx_packet, 512, false);
}

pub fn test_echo_server_odd_src_port(dev: &mut tun::platform::Device, queue: usize) {
    let mut tx_buf = [0 as u8; 1024];
    let mut tx_packet = build_tx_packet(dev.name(), &mut tx_buf, 8001).unwrap();

    test_echo_server_with_tx_packet(dev, queue, &mut tx_packet, 512, false);
}

pub fn test_echo_server_repeated(dev: &mut tun::platform::Device, queue: usize) {
    let mut tx_buf = [0 as u8; 1024];
    let mut tx_packet = build_tx_packet(dev.name(), &mut tx_buf, 8000).unwrap();

    test_echo_server_with_tx_packet(dev, queue, &mut tx_packet, 1, true);
}

fn build_tx_packet<'a>(
    interface: &str,
    buf: &'a mut [u8],
    src_port: u16,
) -> anyhow::Result<PacketBufMut<'a>> {
    let mut packet_buf = PacketBufMut::from_slice(buf);

    EthHdr::with_packet_buf(&mut packet_buf)?
        .set_src_address([0, 1, 2, 3, 4, 5])
        .set_dst_address(net::utils::get_phy_mac_addr(interface).unwrap())
        .ip4();

    Ip4Hdr::with_packet_buf(&mut packet_buf)?
        .set_total_length(32)
        .udp()
        .set_src_address(Ipv4Addr::new(192, 18, 42, 1))
        .set_dst_address(Ipv4Addr::new(192, 18, 42, 42))
        .calc_checksum();

    UdpHdr::with_packet_buf(&mut packet_buf)?
        .set_src_port(src_port)
        .set_dst_port(1234)
        .set_length(12);

    let payload = packet_buf.get_bytes_mut(4).unwrap();
    payload.copy_from_slice(b"lol\n");

    Ok(packet_buf)
}

fn assert_echo_response(tx_packet: &mut PacketBufMut, rx_packet: &mut PacketBufMut) {
    let tx_eth = EthHdr::from_packet_buf(tx_packet).unwrap();
    let rx_eth = EthHdr::from_packet_buf(rx_packet).unwrap();
    assert_eq!(tx_eth.src_address, rx_eth.dst_address);
    assert_eq!(tx_eth.dst_address, rx_eth.src_address);
    assert_eq!(tx_eth.eth_address, rx_eth.eth_address);

    let tx_ip = Ip4Hdr::from_packet_buf(tx_packet).unwrap();
    let rx_ip = Ip4Hdr::from_packet_buf(rx_packet).unwrap();
    assert_eq!(tx_ip.proto, rx_ip.proto);
    assert_eq!(tx_ip.src_addr, rx_ip.dst_addr);
    assert_eq!(tx_ip.dst_addr, rx_ip.src_addr);

    let tx_udp = UdpHdr::from_packet_buf(tx_packet).unwrap();
    let rx_udp = UdpHdr::from_packet_buf(rx_packet).unwrap();
    assert_eq!(tx_udp.src_port, rx_udp.dst_port);
    assert_eq!(tx_udp.dst_port, rx_udp.src_port);

    let tx_payload = tx_packet.get_bytes(4).unwrap();
    let rx_payload = rx_packet.get_bytes(4).unwrap();

    assert_eq!(tx_payload, rx_payload);
}

fn test_echo_server_with_tx_packet(
    dev: &mut tun::platform::Device,
    queue: usize,
    tx_packet: &mut PacketBufMut,
    packet_count: usize,
    repeated: bool,
) {
    for _ in 0..packet_count {
        let tx_amount = dev
            .queue(queue)
            .unwrap()
            .write(tx_packet.as_slice())
            .unwrap();

        assert_eq!(tx_amount, tx_packet.as_slice().len());

        test_rx_packet(dev, tx_packet, queue);

        if repeated {
            test_rx_packet(dev, tx_packet, queue);
        }
    }
}

fn test_rx_packet(dev: &mut tun::platform::Device, tx_packet: &mut PacketBufMut, queue: usize) {
    let mut rx_frame = [0; 1024];
    let rx_amount = dev.queue(queue).unwrap().read(&mut rx_frame).unwrap();

    assert_eq!(tx_packet.as_slice().len(), rx_amount);

    tx_packet.seek(0).unwrap();
    let mut rx_packet = PacketBufMut::from_raw_parts(rx_frame.as_mut_ptr(), rx_amount);

    assert_echo_response(tx_packet, &mut rx_packet);
}
