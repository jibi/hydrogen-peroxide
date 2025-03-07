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

use std::sync::{Arc, RwLock};

use crate::net;

pub struct EchoApp {
    repeat:          bool,
    netstack_handle: Arc<RwLock<dyn net::app::Handle>>,
}

impl EchoApp {
    pub fn new(netstack_handle: Arc<RwLock<dyn net::app::Handle>>, repeat: bool) -> Self {
        EchoApp {
            netstack_handle,
            repeat,
        }
    }

    fn send_echo_response(
        netstack_handle: &mut dyn net::app::Handle,
        socket: &net::app::Socket,
        rx_payload: &[u8],
    ) -> anyhow::Result<()> {
        let mut tx_payload = netstack_handle.new_tx_payload_buf()?;

        tx_payload
            .packet_buf()
            .get_bytes_mut(rx_payload.len())?
            .copy_from_slice(rx_payload);

        netstack_handle.send_payload(socket, &mut tx_payload)?;

        Ok(())
    }

    fn schedule_echo_response(
        netstack_handle: Arc<RwLock<dyn net::app::Handle>>,
        socket: &net::app::Socket,
        rx_payload: &[u8],
    ) -> anyhow::Result<()> {
        let socket = socket.clone();
        let rx_payload: Box<[u8]> = rx_payload.into();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));

            let mut netstack_handle = netstack_handle.write().unwrap();
            EchoApp::send_echo_response(&mut *netstack_handle, &socket, &rx_payload).unwrap();
        });

        Ok(())
    }
}

impl net::app::App for EchoApp {
    fn rx_payload(
        &mut self,
        netstack_handle: &mut dyn net::app::Handle,
        socket: &net::app::Socket,
        rx_payload: &mut [u8],
    ) -> anyhow::Result<()> {
        EchoApp::send_echo_response(netstack_handle, socket, rx_payload)?;
        if self.repeat {
            EchoApp::schedule_echo_response(self.netstack_handle.clone(), socket, rx_payload)?;
        }

        Ok(())
    }
}
