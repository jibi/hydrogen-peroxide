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

//! XSK producer and consumer rings.

use std::{rc::Rc, sync::RwLock};

use crate::{
    xsk,
    xsk::{Desc, FrameAllocator},
};

/// An `xsk_ring_prod` wrapper.
pub struct ProdRing {
    ring:            xsk::sys::xsk_ring_prod,
    frame_allocator: Rc<RwLock<FrameAllocator>>,
    size:            usize,
}

/// An `xsk_ring_cons` wrapper.
pub struct ConsRing {
    ring:            xsk::sys::xsk_ring_cons,
    frame_allocator: Rc<RwLock<FrameAllocator>>,
    size:            usize,
}

impl ProdRing {
    /// Wraps an `xsk_ring_prod` ring around a new [`ProdRing`] object.
    pub fn new_from_xsk_ring_prod(
        frame_allocator: Rc<RwLock<FrameAllocator>>,
        ring: xsk::sys::xsk_ring_prod,
        size: usize,
    ) -> Self {
        ProdRing {
            ring,
            frame_allocator,
            size,
        }
    }

    /// Sets the address of the packet buffer for the descriptor with index `idx`.
    pub fn fill_addr(&mut self, idx: u32, addr: u64) {
        unsafe {
            let addr_ptr = xsk::sys::xsk_ring_prod__fill_addr(&mut self.ring, idx);
            *addr_ptr = addr;
        }
    }

    /// Reserves `num_bufs` buffers in the ring (i.e. increment the `cached_prod` pointer of the ring
    /// by `num_bufs`) and sets `idx` to the index of the first buffer reserved.
    pub fn reserve(&mut self, num_bufs: usize, idx: &mut u32) -> usize {
        unsafe { xsk::sys::xsk_ring_prod__reserve(&mut self.ring, num_bufs, idx) }
    }

    /// Submits `num_bufs` buffers (i.e. increment the `producer` pointer of the ring by `num_bufs`).
    pub fn submit(&mut self, num_bufs: usize) {
        unsafe {
            xsk::sys::xsk_ring_prod__submit(&mut self.ring, num_bufs);
        }
    }

    /// Frees `num_bufs` descriptors.
    pub fn free(&mut self, num_bufs: usize) -> usize {
        unsafe { xsk::sys::xsk_prod_nb_free(&mut self.ring, num_bufs as u32) as usize }
    }

    /// Returns the descriptor with index `idx`.
    pub fn get_desc(&mut self, idx: u32) -> Desc {
        let desc = unsafe { xsk::sys::xsk_ring_prod__tx_desc(&mut self.ring, idx) };

        Desc::new_from_xdp_desc(
            self.frame_allocator.clone(),
            desc,
            (idx as usize) % self.size,
        )
    }

    /// Returns wether the ring needs to be woken up or not.
    pub fn needs_wakeup(&mut self) -> bool {
        unsafe { xsk::sys::xsk_ring_prod__needs_wakeup(&mut self.ring) != 0 }
    }
}

impl ConsRing {
    /// Wraps an `xsk_ring_cons` ring around a new [`ConsRing`] object.
    pub fn new_from_xsk_ring_cons(
        frame_allocator: Rc<RwLock<FrameAllocator>>,
        ring: xsk::sys::xsk_ring_cons,
        size: usize,
    ) -> Self {
        ConsRing {
            ring,
            frame_allocator,
            size,
        }
    }

    /// Peeks up to `num_bufs` descriptors and set `idx` to the index of the first available
    /// buffer.
    pub fn peek(&mut self, num_bufs: usize, idx: &mut u32) -> usize {
        unsafe { xsk::sys::xsk_ring_cons__peek(&mut self.ring, num_bufs, idx) }
    }

    /// Release `num_bufs` (i.e. increments the `consumer` pointer by `num_bufs`).
    pub fn release(&mut self, num_bufs: usize) {
        unsafe {
            xsk::sys::xsk_ring_cons__release(&mut self.ring, num_bufs);
        }
    }

    /// Returns the descriptor with index `idx`.
    pub fn get_desc(&mut self, idx: u32) -> Desc {
        let desc = unsafe { xsk::sys::xsk_ring_cons__rx_desc(&mut self.ring, idx) };

        Desc::new_from_xdp_desc(
            self.frame_allocator.clone(),
            desc,
            (idx as usize) % self.size,
        )
    }
}
