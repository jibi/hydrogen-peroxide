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

use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{Arc, RwLock},
};

pub mod error;
pub use self::error::*;

pub mod configuration;
pub use self::configuration::*;

pub mod packet;
pub use self::packet::*;

pub mod packet_buf;
pub use self::packet_buf::*;

pub mod input;
pub use self::input::*;

pub mod output;
pub use self::output::*;

pub mod utils;

pub mod eth;
pub use self::eth::*;

pub mod arp;
pub use self::arp::*;

pub mod ip4;
pub use self::ip4::*;

pub mod udp;
pub use self::udp::*;

pub mod app;

use crate::xsk;

pub struct NetStack {
    configuration: Configuration,
    xsk_handle:    Arc<RwLock<xsk::net::Handle>>,

    iface_mac:    [u8; 6],
    bind_address: Ipv4Addr,
    bind_port:    u16,

    arp_table: HashMap<Ipv4Addr, [u8; 6]>,
}

unsafe impl Send for NetStack {}
unsafe impl Sync for NetStack {}

pub struct Net {
    app:      Box<dyn app::App>,
    netstack: Arc<RwLock<NetStack>>,
}

unsafe impl Send for Net {}
unsafe impl Sync for Net {}

impl Net {
    pub fn new(mut configuration: Configuration) -> Self {
        let xsk_handle = Arc::new(RwLock::new(configuration.take_xsk_handle()));

        let (interface, bind_address, bind_port) = {
            let xsk_handle = xsk_handle.read().unwrap();
            let cfg = xsk_handle.configuration();

            (
                String::from(cfg.interface()),
                cfg.bind_address(),
                cfg.bind_port(),
            )
        };

        let iface_mac = utils::get_phy_mac_addr(&interface).unwrap();

        let netstack = Arc::new(RwLock::new(NetStack {
            configuration,
            xsk_handle,

            iface_mac,
            bind_address,
            bind_port,

            arp_table: HashMap::new(),
        }));

        let app = {
            let n = netstack.clone();
            let netstack = netstack.write().unwrap();
            (netstack.configuration.app_allocator())(n)
        };

        Net { netstack, app }
    }
}

impl xsk::net::Net for Net {
    fn rx_packet(&mut self, desc: xsk::Desc) -> anyhow::Result<()> {
        self.do_rx_packet(desc)?;
        Ok(())
    }
}
