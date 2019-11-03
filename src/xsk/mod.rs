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

//! Run XSK on a given interface.

#![warn(missing_docs)]

mod configuration;
pub use self::configuration::*;

mod desc;
pub use self::desc::*;

mod error;
use self::error::*;

mod frame_allocator;
use self::frame_allocator::*;

mod ring;
use self::ring::*;

mod queue;
use self::queue::*;

mod socket;
pub use self::socket::TxSocket;
use self::socket::*;

pub mod net;

mod sys;

mod umem;
use self::umem::*;

mod xdp_prog;
use self::xdp_prog::*;

use std::{
    fs::File,
    io::prelude::*,
    os::unix::io::FromRawFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

/// Controls how many packets in a row can be received and transmitted.
pub const BATCH_SIZE: usize = 64;

/// The main XSK object.
#[allow(dead_code)]
pub struct Xsk {
    xdp_prog:       XdpProg,
    threads_runner: ThreadsRunner,
}

unsafe impl Send for Xsk {}

impl Xsk {
    /// Creates a new [`Xsk`] object.
    pub fn new(configuration: Configuration) -> Result<Self> {
        let configuration = Arc::new(configuration);

        let mut threads_runner = ThreadsRunner::new();
        let mut queues = Queues::default();

        for queue_num in configuration.queues() {
            let cfg = configuration.clone();
            queues.push(Queue::new(cfg, *queue_num, &threads_runner)?);
        }

        let xdp_prog = XdpProg::load(&configuration, &queues)?;

        for (socket_idx, mut socket) in QueuesSockets::from(queues).into_iter().enumerate() {
            let net = (configuration.net_allocator())(socket.take_tx_socket().into());

            threads_runner.spawn(format!("socket {} RX loop", socket_idx), move |runner| {
                RxSocket::rx_loop(runner, net, socket.take_rx_socket())
            });
        }

        Ok(Xsk {
            xdp_prog,
            threads_runner,
        })
    }

    /// Returns the runner associated with the XSK object.
    pub fn runner(&mut self) -> Runner {
        self.threads_runner.runner.clone()
    }

    /// Waits for all XSK threads to terminate.
    pub fn wait_for_threads(&mut self) {
        while let Some(t) = self.threads_runner.threads.pop() {
            t.join().unwrap();
        }
    }

    /// Sets the required rlimit for eBPF.
    pub fn set_rlimit() -> Result<()> {
        let rlim = libc::rlimit {
            rlim_cur: libc::RLIM_INFINITY,
            rlim_max: libc::RLIM_INFINITY,
        };

        let errno = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
        if errno != 0 {
            return Err(Error::SetrlimitFailed(errno));
        }

        Ok(())
    }
}

impl Drop for Xsk {
    fn drop(&mut self) {
        self.threads_runner.runner.stop();
    }
}

/// A type for keeping track of the state and halting the executing of the XSK threads.
#[derive(Clone)]
pub struct Runner {
    running:  Arc<AtomicBool>,
    pipe_fds: [i32; 2],
}

impl Default for Runner {
    fn default() -> Self {
        let mut pipe_fds = [0 as libc::c_int; 2];
        unsafe {
            libc::pipe(pipe_fds.as_mut_ptr());
        }

        Runner {
            running: Arc::new(AtomicBool::new(true)),
            pipe_fds,
        }
    }
}

impl Runner {
    /// Returns true if the runner is in running state.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Returns the reader fd of the runner's control pipe.
    pub fn pipe_reader_fd(&self) -> libc::c_int {
        self.pipe_fds[0]
    }

    /// Stops the runner.
    pub fn stop(&mut self) {
        if !self.is_running() {
            return;
        }

        self.running.store(false, Ordering::SeqCst);

        let mut writer = unsafe { File::from_raw_fd(self.pipe_fds[1]) };
        writer.write_all(b"kthxbye").unwrap();
    }
}

/// A type for keeping track of the XSK operation related threads.
pub struct ThreadsRunner {
    runner:  Runner,
    threads: Vec<thread::JoinHandle<()>>,
}

impl ThreadsRunner {
    fn new() -> Self {
        ThreadsRunner {
            runner:  Runner::default(),
            threads: Vec::new(),
        }
    }

    fn spawn<F>(&mut self, name: String, func: F)
    where
        F: FnOnce(Runner),
        F: Send + 'static,
    {
        let r = self.runner.clone();

        self.threads
            .push(thread::Builder::new().name(name).spawn(|| func(r)).unwrap());
    }
}
