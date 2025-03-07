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

//! UMEM sockets.

use std::{mem, rc::Rc, sync::RwLock};

use crate::{
    xsk,
    xsk::{
        Configuration, ConsRing, Error::*, FrameAllocator, NeedsWakeup, ProdRing, Result, RxSocket,
    },
};

/// An UMEM socket.
pub struct Umem {
    pub frame_allocator: Rc<RwLock<FrameAllocator>>,
    pub umem:            *mut xsk::sys::xsk_umem,

    cq: ConsRing,
    fq: ProdRing,

    needs_wakeup: NeedsWakeup,
}

unsafe impl Send for Umem {}

impl Umem {
    pub fn size(cfg: &Rc<Configuration>) -> usize {
        (cfg.rx_size() * cfg.socks_per_queue() + cfg.tx_size() * cfg.socks_per_queue())
            * cfg.frame_size()
    }

    /// Creates a new [`Umem`] object.
    pub fn new(cfg: &Rc<Configuration>) -> Result<Self> {
        let rx_size = cfg.rx_size() * cfg.socks_per_queue();
        let tx_size = cfg.tx_size() * cfg.socks_per_queue();

        // Initialize the frame allocator.
        let frame_allocator = Rc::new(RwLock::new(FrameAllocator::new(
            rx_size + tx_size,
            cfg.frame_size(),
        )?));

        // Initialize the umem socket.
        let mut fq_ring: xsk::sys::xsk_ring_prod = unsafe { mem::zeroed() };
        let mut cq_ring: xsk::sys::xsk_ring_cons = unsafe { mem::zeroed() };

        let mut umem_opts: xsk::sys::xsk_umem_opts = unsafe { mem::zeroed() };
        umem_opts.sz = mem::size_of::<xsk::sys::xsk_umem_opts>();
        umem_opts.size = Self::size(cfg) as u64;
        umem_opts.fill_size = rx_size as u32;
        umem_opts.comp_size = tx_size as u32;
        umem_opts.frame_size = cfg.frame_size() as u32;

        let umem = {
            let frame_allocator = frame_allocator.write().unwrap();

            unsafe {
                xsk::sys::xsk_umem__create_opts(
                    frame_allocator.buffer,
                    &mut fq_ring,
                    &mut cq_ring,
                    &mut umem_opts,
                )
            }
        };

        if umem.is_null() {
            return Err(XskUmemCreateFailed(nix::errno::Errno::last_raw()));
        }

        // Initialize the complete ring.
        let cq = ConsRing::new_from_xsk_ring_cons(frame_allocator.clone(), cq_ring, tx_size);

        // Initialize and populate the fill ring.
        let mut fq = ProdRing::new_from_xsk_ring_prod(frame_allocator.clone(), fq_ring, rx_size);
        {
            let mut frame_allocator = frame_allocator.write().unwrap();

            let mut rx_idx = 0;
            let n = fq.reserve(rx_size, &mut rx_idx);
            if n != rx_size {
                return Err(XskFqRingProdReserveFailed);
            }

            for i in 0..rx_size {
                let addr = frame_allocator.alloc_frame().unwrap();
                fq.fill_addr(i as u32, addr);
            }

            fq.submit(rx_size);
        }

        Ok(Umem {
            frame_allocator,

            fq,
            cq,
            umem,
            needs_wakeup: cfg.needs_wakeup(),
        })
    }

    /// Reclaim `num_bufs` descriptor in the FQ UMEM ring.
    pub fn reclaim_fq_bufs(&mut self, socket: &mut RxSocket, num_bufs: usize) -> Result<()> {
        if num_bufs == 0 {
            if self.needs_wakeup.value && self.fq.needs_wakeup() {
                socket.poll()?;
            }

            return Ok(());
        }

        if self.fq.free(xsk::BATCH_SIZE) > 0 {
            let mut idx_fq = 0;

            let mut ret = self.fq.reserve(num_bufs, &mut idx_fq);

            while ret != num_bufs {
                if self.needs_wakeup.value && self.fq.needs_wakeup() {
                    socket.poll()?;
                }

                ret = self.fq.reserve(num_bufs, &mut idx_fq);
            }

            self.fq.submit(num_bufs);
        }

        Ok(())
    }

    /// Reclaim `num_bufs` descriptors in the CQ UMEM ring.
    pub fn reclaim_cq_bufs(&mut self, num_bufs: usize) {
        let mut tx_idx = 0;
        let completed = self.cq.peek(num_bufs, &mut tx_idx);
        if completed > 0 {
            self.cq.release(completed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new() {
        let mut cfg = Configuration::default();
        cfg.set_needs_wakeup(NeedsWakeup::new(false));

        let umem = Umem::new(&Rc::new(cfg));
        assert!(umem.is_ok());
    }

    #[test]
    fn test_invalid_ring_size() {
        let mut cfg = Configuration::default();
        cfg.set_rx_size(42);
        cfg.set_needs_wakeup(NeedsWakeup::new(false));

        let umem = Umem::new(&Rc::new(cfg));
        assert!(umem.is_err());
    }
}
