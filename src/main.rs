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

#[macro_use]
extern crate log;

use clap::{ArgAction, Parser};
use simple_signal::Signal;

use std::{
    net::Ipv4Addr,
    num::ParseIntError,
    sync::{Arc, RwLock},
};

use libh2o2::{echo, net, xsk};

#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "Gilberto Bertin <me@jibi.io>")]
pub struct Args {
    /// Sets the interface
    #[arg(short = 'i', long = "interface")]
    pub interface: String,

    /// Sets the bind address
    #[arg(short = 'a', long = "address")]
    pub bind_address: Ipv4Addr,

    /// Sets the bind port
    #[arg(short = 'p', long = "port")]
    pub bind_port: u16,

    /// Sets the XDP program path
    #[arg(long = "xdp-prog-path")]
    pub xdp_prog_path: Option<String>,

    /// Run on given queue
    #[arg(long = "queue")]
    pub queue: Option<Vec<usize>>,

    /// Sets the number of XSK socks per queue
    #[arg(long = "socks-per-queue", value_parser = validate_socks_per_queue)]
    pub socks_per_queue: Option<usize>,

    /// Sets the RX ring size
    #[arg(long = "rx-size")]
    pub rx_size: Option<usize>,

    /// Sets the TX ring size
    #[arg(long = "tx-size")]
    pub tx_size: Option<usize>,

    /// Sets the frame size
    #[arg(long = "frame-size")]
    pub frame_size: Option<usize>,

    /// Sets the xsk mode of operation
    #[arg(long = "xsk-mode")]
    pub xsk_mode: Option<xsk::XskMode>,

    /// Disable the XDP_NEEDS_WAKEUP flag (required for kernels < 4.4)
    #[arg(long = "no-needs-wakeup", action=ArgAction::SetTrue, value_parser = parse_no_needs_wakeup)]
    pub needs_wakeup: xsk::NeedsWakeup,
}

fn validate_socks_per_queue(socks: &str) -> Result<usize, String> {
    let val: usize = socks.parse().map_err(|e: ParseIntError| e.to_string())?;
    if val != 0 && (val & (val - 1)) == 0 {
        Ok(val)
    } else {
        Err("not a power of 2".to_string())
    }
}

fn parse_no_needs_wakeup(val: &str) -> Result<xsk::NeedsWakeup, String> {
    val.parse::<bool>()
        .map(xsk::NeedsWakeup::new)
        .map_err(|e| e.to_string())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let xsk_cfg = build_xsk_config(&args);

    xsk::Xsk::set_rlimit().unwrap_or_else(|err| {
        error!("{}", err);
        std::process::exit(1);
    });

    let mut xsk = xsk::Xsk::new(xsk_cfg).unwrap_or_else(|err| {
        error!("{}", err);
        std::process::exit(1);
    });

    let runner = xsk.runner();
    simple_signal::set_handler(&[Signal::Int, Signal::Term], move |_signals| {
        runner.clone().stop();
    });

    info!(
        "Listening on {}, {}:{}",
        args.interface, args.bind_address, args.bind_port
    );

    xsk.wait_for_threads();
}

fn build_xsk_config(args: &Args) -> xsk::Configuration {
    let mut cfg = xsk::Configuration::default();

    let net_allocator: Box<xsk::net::NetAllocator> = Box::new(|xsk_handle: xsk::net::Handle| {
        let app_allocator: Box<net::app::AppAllocator> =
            Box::new(|net_handle: Arc<RwLock<dyn net::app::Handle>>| {
                Box::new(echo::EchoApp::new(net_handle, false))
            });

        let mut net_cfg = net::Configuration::default();

        net_cfg
            .set_app_allocator(app_allocator)
            .set_xsk_handle(xsk_handle);

        Box::new(net::Net::new(net_cfg))
    });

    cfg.set_interface(&args.interface)
        .set_bind_address(args.bind_address)
        .set_bind_port(args.bind_port)
        .set_net_allocator(net_allocator);

    if let Some(v) = args.xdp_prog_path.as_ref() {
        cfg.set_xdp_prog_path(v);
    }

    if let Some(v) = args.queue.as_ref() {
        cfg.set_queues(v.clone());
    }

    if let Some(v) = args.socks_per_queue {
        cfg.set_socks_per_queue(v);
    }

    if let Some(v) = args.rx_size {
        cfg.set_rx_size(v);
    }

    if let Some(v) = args.tx_size {
        cfg.set_tx_size(v);
    }

    if let Some(v) = args.frame_size {
        cfg.set_frame_size(v);
    }

    if let Some(v) = args.xsk_mode {
        cfg.set_mode(v);
    }

    cfg.set_needs_wakeup(args.needs_wakeup);

    cfg
}
