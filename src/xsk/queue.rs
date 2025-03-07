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

//! XSK RX/TX queue.

use std::{rc::Rc, sync::RwLock};

use crate::xsk::{Configuration, Result, Socket, ThreadsRunner, Umem};

/// A collection of XSK queues.
pub struct Queues(Vec<Queue>);

impl Default for Queues {
    /// Returns a new empty [`Queues`] object.
    fn default() -> Self {
        Queues(Vec::new())
    }
}

impl Queues {
    /// Adds a queue.
    pub fn push(&mut self, queue: Queue) {
        self.0.push(queue);
    }
}

/// A collection of XSK sockets belonging to the same Queue.
pub struct Queue {
    pub sockets: Vec<Socket>,
}

/// An XSK RX/TX queue.
///
/// It may be made up of multiple XSK sockets.
impl Queue {
    /// Creates a new XSK [`Queue`].
    pub fn new(
        cfg: Rc<Configuration>,
        queue_num: usize,
        threads_runner: &ThreadsRunner,
    ) -> Result<Self> {
        let mut umem = Rc::new(RwLock::new(Umem::new(&cfg)?));

        let mut sockets = Vec::new();
        for _ in 0..cfg.socks_per_queue() {
            let socket = Socket::new(
                cfg.clone(),
                &mut umem,
                queue_num,
                threads_runner.runner.pipe_reader_fd(),
            )?;

            sockets.push(socket);
        }

        Ok(Queue { sockets })
    }
}

/// A wrapper type around [`Queues`] used to provide an iterator to iterate through each socket in each of the queues.
pub struct QueuesSockets(Queues);

impl From<Queues> for QueuesSockets {
    fn from(v: Queues) -> Self {
        QueuesSockets(v)
    }
}

impl IntoIterator for QueuesSockets {
    type Item = Socket;

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let queue_iter = (self.0).0.into_iter();
        let socket_iter = None;

        IntoIter {
            queue_iter,
            socket_iter,
        }
    }
}

pub struct IntoIter {
    queue_iter:  std::vec::IntoIter<Queue>,
    socket_iter: Option<std::iter::Peekable<std::vec::IntoIter<Socket>>>,
}

impl Iterator for IntoIter {
    type Item = Socket;

    fn next(&mut self) -> Option<Socket> {
        if self.socket_iter.is_none() || self.socket_iter.as_mut().unwrap().peek().is_none() {
            if let Some(next_queue) = self.queue_iter.next() {
                self.socket_iter = Some(next_queue.sockets.into_iter().peekable())
            } else {
                return None;
            }
        }

        if let Some(socket_iter) = self.socket_iter.as_mut() {
            return socket_iter.next();
        };

        None
    }
}

/// A wrapper type around &[`Queues`] used to provide an iterator to iterate through each socket in each of the queues.
pub struct QueuesSocketsRef<'a>(&'a Queues);

impl<'a> From<&'a Queues> for QueuesSocketsRef<'a> {
    fn from(v: &'a Queues) -> Self {
        QueuesSocketsRef(v)
    }
}

impl<'a> IntoIterator for &'a QueuesSocketsRef<'a> {
    type Item = &'a Socket;

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let queue_iter = (self.0).0.iter();
        let socket_iter = None;

        Iter {
            queue_iter,
            socket_iter,
        }
    }
}

pub struct Iter<'a> {
    queue_iter:  std::slice::Iter<'a, Queue>,
    socket_iter: Option<std::iter::Peekable<std::slice::Iter<'a, Socket>>>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Socket;

    fn next(&mut self) -> Option<&'a Socket> {
        if self.socket_iter.is_none() || self.socket_iter.as_mut().unwrap().peek().is_none() {
            if let Some(next_queue) = self.queue_iter.next() {
                self.socket_iter = Some(next_queue.sockets.as_slice().iter().peekable())
            } else {
                return None;
            }
        }

        if let Some(socket_iter) = self.socket_iter.as_mut() {
            return socket_iter.next();
        };

        None
    }
}
