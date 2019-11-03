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

//! XDP descriptor.

use std::sync::{Arc, RwLock};

use crate::{xsk, xsk::FrameAllocator};

/// An `xdp_desc` descriptor belonging to a [`ProdRing`](crate::xsk::ring::ProdRing) or [`ConsRing`](crate::xsk::ring::ConsRing) ring.
///
/// A descriptor contains:
/// * an address used to reference a particular frame in the UMEM memory buffer
/// * the length of the frame
pub struct Desc {
    frame_allocator: Arc<RwLock<FrameAllocator>>,
    desc:            *mut xsk::sys::xdp_desc,
    index:           usize,
}

impl Desc {
    /// Wraps an `xdp_desc` descriptor around a new [`Desc`] object.
    pub fn new_from_xdp_desc(
        frame_allocator: Arc<RwLock<FrameAllocator>>,
        desc: *mut xsk::sys::xdp_desc,
        index: usize,
    ) -> Self {
        Desc {
            frame_allocator,
            desc,
            index,
        }
    }

    /// Returns a pointer to the descriptor's packet buffer.
    pub fn packet(&self) -> *mut u8 {
        let frame_allocator = self.frame_allocator.read().unwrap();

        unsafe {
            let addr = (*self.desc).addr;
            xsk::sys::xsk_umem__get_data(frame_allocator.buffer, addr) as *mut u8
        }
    }

    /// Returns the length of the descriptor's packet buffer.
    pub fn len(&self) -> usize {
        unsafe { (*self.desc).len as usize }
    }

    /// Returns true if the descriptor's packet buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets the len of the descriptor's packet buffer.
    pub fn set_len(&mut self, len: usize) {
        unsafe { (*self.desc).len = len as u32 }
    }

    /// Return the position of the descriptor inside the ring.
    pub fn index(&self) -> usize {
        self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_new() {
        let frame_allocator = FrameAllocator::new(4096, 4096);
        assert!(frame_allocator.is_ok());

        let frame_allocator = Arc::new(RwLock::new(frame_allocator.unwrap()));

        let mut xdp_desc = xsk::sys::xdp_desc {
            addr:    0,
            len:     54,
            options: 0,
        };

        let desc = Desc::new_from_xdp_desc(frame_allocator.clone(), &mut xdp_desc, 0);
        assert_eq!(desc.desc, &mut xdp_desc as *mut xsk::sys::xdp_desc);
    }

    #[test]
    fn test_packet() {
        let frame_allocator = FrameAllocator::new(4096, 4096);
        assert!(frame_allocator.is_ok());

        let frame_allocator = Arc::new(RwLock::new(frame_allocator.unwrap()));

        let mut xdp_desc = xsk::sys::xdp_desc {
            addr:    0,
            len:     54,
            options: 0,
        };

        let mut desc = Desc::new_from_xdp_desc(frame_allocator.clone(), &mut xdp_desc, 0);
        assert_ne!(desc.packet(), ptr::null_mut());

        desc.set_len(80);
        assert_eq!(desc.len(), 80);
    }
}
