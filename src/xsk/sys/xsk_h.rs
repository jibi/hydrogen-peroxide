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

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::xsk::sys::bindings::*;
use std::sync::atomic::{fence, Ordering};

#[inline(always)]
fn libbpf_smp_rmb() {
    fence(Ordering::Acquire);
}

#[inline(always)]
fn libbpf_smp_wmb() {
    fence(Ordering::Release);
}

#[inline(always)]
fn libbpf_smp_rwmb() {
    fence(Ordering::AcqRel);
}

#[inline(always)]
pub unsafe fn xsk_ring_prod__fill_addr(fill: *mut xsk_ring_prod, idx: u32) -> *mut u64 {
    let addrs = (*fill).ring as *mut u64;

    addrs.offset((idx & (*fill).mask) as isize)
}

#[inline(always)]
#[allow(dead_code)]
pub unsafe fn xsk_ring_cons__comp_addr(comp: *mut xsk_ring_cons, idx: u32) -> *mut u64 {
    let addrs = (*comp).ring as *mut u64;

    addrs.offset((idx & (*comp).mask) as isize)
}

#[inline(always)]
#[allow(dead_code)]
pub unsafe fn xsk_ring_prod__tx_desc(tx: *mut xsk_ring_prod, idx: u32) -> *mut xdp_desc {
    let descs = (*tx).ring as *mut xdp_desc;

    descs.offset((idx & (*tx).mask) as isize)
}

#[inline(always)]
pub unsafe fn xsk_ring_cons__rx_desc(rx: *mut xsk_ring_cons, idx: u32) -> *mut xdp_desc {
    let descs = (*rx).ring as *mut xdp_desc;

    descs.offset((idx & (*rx).mask) as isize)
}

#[inline(always)]
pub unsafe fn xsk_ring_prod__needs_wakeup(r: *mut xsk_ring_prod) -> u32 {
    *((*r).flags) & XDP_RING_NEED_WAKEUP
}

#[inline(always)]
pub unsafe fn xsk_prod_nb_free(r: *mut xsk_ring_prod, nb: u32) -> u32 {
    let free_entries = (*r).cached_cons - (*r).cached_prod;

    if free_entries >= nb {
        return free_entries;
    }

    (*r).cached_cons = *(*r).consumer + (*r).size;

    (*r).cached_cons - (*r).cached_prod
}

#[inline(always)]
pub unsafe fn xsk_cons_nb_avail(r: *mut xsk_ring_cons, nb: u32) -> u32 {
    let mut entries = (*r).cached_prod - (*r).cached_cons;
    if entries == 0 {
        (*r).cached_prod = *(*r).producer;
        entries = (*r).cached_prod - (*r).cached_cons;
    }

    if entries > nb {
        nb
    } else {
        entries
    }
}

#[inline(always)]
pub unsafe fn xsk_ring_prod__reserve(prod: *mut xsk_ring_prod, nb: usize, idx: *mut u32) -> usize {
    if xsk_prod_nb_free(prod, nb as u32) < nb as u32 {
        return 0;
    }

    *idx = (*prod).cached_prod;
    (*prod).cached_prod += nb as u32;

    nb
}

#[inline(always)]
pub unsafe fn xsk_ring_prod__submit(prod: *mut xsk_ring_prod, nb: usize) {
    libbpf_smp_wmb();

    *(*prod).producer += nb as u32;
}

#[inline(always)]
pub unsafe fn xsk_ring_cons__peek(cons: *mut xsk_ring_cons, nb: usize, idx: *mut u32) -> usize {
    let entries = xsk_cons_nb_avail(cons, nb as u32);

    if entries > 0 {
        libbpf_smp_rmb();

        *idx = (*cons).cached_cons;
        (*cons).cached_cons += entries;
    }

    entries as usize
}

#[inline(always)]
pub unsafe fn xsk_ring_cons__release(cons: *mut xsk_ring_cons, nb: usize) {
    libbpf_smp_rwmb();

    *(*cons).consumer += nb as u32;
}

#[inline(always)]
pub unsafe fn xsk_umem__get_data(umem_area: *mut libc::c_void, addr: u64) -> *mut libc::c_void {
    (umem_area as *mut u8).offset(addr as isize) as *mut libc::c_void
}
