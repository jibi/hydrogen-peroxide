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

//! A type for dealing with XSK configuration.

use std::{net::Ipv4Addr, str::FromStr};

use crate::{
    xsk,
    xsk::{net::NetAllocator, Error, Result},
};

/// Configuration builder for an XSK object.
pub struct Configuration {
    interface:     Option<String>,
    address:       Option<Ipv4Addr>,
    port:          Option<u16>,
    net_allocator: Option<Box<NetAllocator>>,

    xdp_prog_path:   String,
    queues:          Vec<usize>,
    socks_per_queue: usize,
    rx_size:         usize,
    tx_size:         usize,
    frame_size:      usize,
    mode:            XskMode,
    needs_wakeup:    NeedsWakeup,
}

impl Default for Configuration {
    /// Creates a new [`Configuration`] object with the default values.
    fn default() -> Self {
        Configuration {
            interface:     None,
            address:       None,
            port:          None,
            net_allocator: None,

            xdp_prog_path:   "./kern/xsk_kern.o".to_string(),
            queues:          vec![0],
            socks_per_queue: 1,
            rx_size:         xsk::sys::XSK_RING_PROD__DEFAULT_NUM_DESCS as usize,
            tx_size:         xsk::sys::XSK_RING_PROD__DEFAULT_NUM_DESCS as usize,
            frame_size:      xsk::sys::XSK_UMEM__DEFAULT_FRAME_SIZE as usize,
            mode:            XskMode::Skb,
            needs_wakeup:    NeedsWakeup::new(true),
        }
    }
}

impl Configuration {
    /// Set the listening interface.
    pub fn set_interface<S: AsRef<str>>(&mut self, name: S) -> &mut Self {
        self.interface = Some(name.as_ref().into());
        self
    }

    /// Get the listening interface
    pub fn interface(&self) -> &str {
        self.interface.as_ref().unwrap()
    }

    /// Set the listening IPv4 address.
    pub fn set_bind_address(&mut self, addr: Ipv4Addr) -> &mut Self {
        self.address = Some(addr);
        self
    }

    /// Get the listening IPv4 address.
    pub fn bind_address(&self) -> Ipv4Addr {
        self.address.unwrap()
    }

    /// Set the listening port.
    pub fn set_bind_port(&mut self, port: u16) -> &mut Self {
        self.port = Some(port);
        self
    }

    /// Get the listening port.
    pub fn bind_port(&self) -> u16 {
        self.port.unwrap()
    }

    /// Set the NetAllocator callback.
    pub fn set_net_allocator(&mut self, net_allocator: Box<NetAllocator>) -> &mut Self {
        self.net_allocator = Some(net_allocator);
        self
    }

    /// Get the RX callback which will handle incoming packets.
    pub fn net_allocator(&self) -> &NetAllocator {
        self.net_allocator.as_ref().unwrap()
    }

    /// Set the path of the XDP program.
    pub fn set_xdp_prog_path<S: AsRef<str>>(&mut self, value: S) -> &mut Self {
        self.xdp_prog_path = value.as_ref().into();
        self
    }

    /// Get the path of the XDP program.
    pub fn xdp_prog_path(&self) -> &str {
        self.xdp_prog_path.as_ref()
    }

    /// Set which queues should be enabled.
    pub fn set_queues(&mut self, value: Vec<usize>) -> &mut Self {
        self.queues = value;
        self
    }

    /// Get which queues should be enabled.
    pub fn queues(&self) -> &[usize] {
        self.queues.as_ref()
    }

    /// Set the number of XSK sockets per queue.
    pub fn set_socks_per_queue(&mut self, value: usize) -> &mut Self {
        self.socks_per_queue = value;
        self
    }

    /// Get the number of XSK sockets per queue.
    pub fn socks_per_queue(&self) -> usize {
        self.socks_per_queue
    }

    /// Set the number of descriptors per RX ring.
    pub fn set_rx_size(&mut self, value: usize) -> &mut Self {
        self.rx_size = value;
        self
    }

    /// Get the number of descriptors per RX ring.
    pub fn rx_size(&self) -> usize {
        self.rx_size
    }

    /// Sets the number of descriptors per TX ring.
    pub fn set_tx_size(&mut self, value: usize) -> &mut Self {
        self.tx_size = value;
        self
    }

    /// Get the number of descriptors per TX ring.
    pub fn tx_size(&self) -> usize {
        self.tx_size
    }

    /// Set the frame size.
    pub fn set_frame_size(&mut self, value: usize) -> &mut Self {
        self.frame_size = value;
        self
    }

    /// Get the frame size.
    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    /// Set the the XSK mode of operation.
    pub fn set_mode(&mut self, value: XskMode) -> &mut Self {
        self.mode = value;
        self
    }

    /// Get the the XSK mode of operation.
    pub fn mode(&self) -> XskMode {
        self.mode
    }

    /// Set the needs_wakeup behaviour.
    pub fn set_needs_wakeup(&mut self, value: NeedsWakeup) -> &mut Self {
        self.needs_wakeup = value;
        self
    }

    /// Get the needs_wakeup behaviour.
    pub fn needs_wakeup(&self) -> NeedsWakeup {
        self.needs_wakeup
    }

    /// Validate configuration.
    ///
    /// This method makes sure all mandatory properties are set.
    pub fn validate(&self) -> Result<()> {
        if self.interface.is_none() {
            return Err(Error::InvalidConfigWithMissingProperty(
                "interface".to_string(),
            ));
        }

        if self.address.is_none() {
            return Err(Error::InvalidConfigWithMissingProperty(
                "bind address".to_string(),
            ));
        }

        if self.port.is_none() {
            return Err(Error::InvalidConfigWithMissingProperty(
                "bind port".to_string(),
            ));
        }

        if self.net_allocator.is_none() {
            return Err(Error::InvalidConfigWithMissingProperty(
                "net allocator".to_string(),
            ));
        }

        Ok(())
    }
}

/// XSK mode of operation.
#[derive(Debug, Copy, Clone)]
pub enum XskMode {
    /// Skb mode.
    Skb,

    /// Driver mode.
    Drv,

    /// Zerocopy driver mode.
    DrvZeroCopy,
}

impl XskMode {
    /// Returns the representation of the XskMode object as XDP flags
    pub fn into_xdp_flags(self) -> u32 {
        match self {
            XskMode::Skb => xsk::sys::XDP_FLAGS_SKB_MODE,
            XskMode::Drv | XskMode::DrvZeroCopy => xsk::sys::XDP_FLAGS_DRV_MODE,
        }
    }

    /// Returns the representation of the XskMode object as XDP bind flags
    pub fn into_bind_flags(self) -> u16 {
        match self {
            XskMode::Skb | XskMode::Drv => xsk::sys::XDP_COPY as u16,
            XskMode::DrvZeroCopy => xsk::sys::XDP_ZEROCOPY as u16,
        }
    }
}

impl FromStr for XskMode {
    type Err = Error;

    /// Creates a new XskMode object from a string.
    ///
    /// Possible values for the input string are `skb`, `drv` and `drv-zc`.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use XskMode::*;

        match s {
            "skb" => Ok(Skb),
            "drv" => Ok(Drv),
            "drv-zc" => Ok(DrvZeroCopy),
            _ => Err(Error::InvalidXskMode),
        }
    }
}

/// Wrapper for the `XDP_USE_NEED_WAKEUP` flag.
#[derive(Debug, Copy, Clone)]
pub struct NeedsWakeup {
    #[allow(missing_docs)]
    pub value: bool,
}

impl NeedsWakeup {
    /// Creates a new NeedsWakeup object.
    pub fn new(value: bool) -> Self {
        NeedsWakeup { value }
    }

    /// Returns the representation of the NeedsWakeup object as XDP bind flags.
    pub fn into_bind_flags(self) -> u16 {
        if self.value {
            xsk::sys::XDP_USE_NEED_WAKEUP as u16
        } else {
            0
        }
    }
}
