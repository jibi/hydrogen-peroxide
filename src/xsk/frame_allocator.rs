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

//! Memory allocator for XSK frames.

use libc::{sysconf, _SC_PAGESIZE};

use std::ptr;

use crate::xsk::{Error::*, Result};

/// A memory allocator to allocate frame buffers for [`ProdRing`](crate::xsk::ring::ProdRing) rings.
pub struct FrameAllocator {
    pub buffer: *mut libc::c_void,

    frame_addr: Vec<u64>,
}

unsafe impl Send for FrameAllocator {}

impl FrameAllocator {
    /// Creates a new [`FrameAllocator`] object.
    pub fn new(num_frames: usize, frame_size: usize) -> Result<Self> {
        let mut buffer: *mut libc::c_void = ptr::null_mut();

        unsafe {
            let page_size = sysconf(_SC_PAGESIZE) as usize;
            let errno = libc::posix_memalign(&mut buffer, page_size, num_frames * frame_size);
            if errno != 0 {
                return Err(FrameAllocatorAllocationFailed(errno));
            }
        }

        let mut frame_addr = Vec::new();
        for i in (0..num_frames).rev() {
            frame_addr.push((i * frame_size) as u64);
        }

        Ok(FrameAllocator { buffer, frame_addr })
    }

    /// Allocates a new frame and return its address.
    pub fn alloc_frame(&mut self) -> Option<u64> {
        self.frame_addr.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new_frame_allocator() {
        let frame_allocator = FrameAllocator::new(4096, 4096);
        assert!(frame_allocator.is_ok());

        let frame_allocator = frame_allocator.unwrap();
        assert_ne!(frame_allocator.buffer, ptr::null_mut());
    }

    #[test]
    fn test_frame_allocator_alloc_frame() {
        let frame_allocator = FrameAllocator::new(2, 4096);
        assert!(frame_allocator.is_ok());

        let mut frame_allocator = frame_allocator.unwrap();

        let frame = frame_allocator.alloc_frame();
        assert!(frame.is_some());

        let frame = frame.unwrap();
        assert_eq!(frame, 0);

        let frame = frame_allocator.alloc_frame();
        assert!(frame.is_some());

        let frame = frame.unwrap();
        assert_eq!(frame, 4096);

        let frame = frame_allocator.alloc_frame();
        assert!(frame.is_none());
    }
}
