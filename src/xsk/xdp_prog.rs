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

//! XSK-specific types for dealing with XDP programs and maps.

use libc::c_void;

use std::{
    ffi::CString,
    net::Ipv4Addr,
    {mem, ptr},
};

use crate::{
    xsk,
    xsk::{Configuration, Error::*, Queues, QueuesSocketsRef, Result},
};

/// An object responsible for managing the lifecycle of an XSK XDP program on a given interface.
pub struct XdpProg {
    iface_index: u32,
}

impl XdpProg {
    /// Load a new XSK XDP program on an interface.
    pub fn load(cfg: &Configuration, queues: &Queues) -> Result<Self> {
        let iface_index = Self::if_nametoindex(cfg.interface().to_string());

        let mut prog_load_attr: xsk::sys::bpf_prog_load_attr = unsafe { mem::zeroed() };
        let mut obj: *mut xsk::sys::bpf_object = ptr::null_mut();
        let mut prog_fd: i32 = 0;

        let file_cstr = CString::new(cfg.xdp_prog_path().to_string()).unwrap();

        prog_load_attr.file = file_cstr.as_ptr();
        prog_load_attr.prog_type = xsk::sys::bpf_prog_type_BPF_PROG_TYPE_XDP;

        let ret = unsafe { xsk::sys::bpf_prog_load_xattr(&prog_load_attr, &mut obj, &mut prog_fd) };
        if ret != 0 {
            return Err(BpfProgLoadFailed(-ret));
        }

        Self::load_xdp_prog_maps(
            obj,
            cfg.bind_address(),
            cfg.bind_port(),
            queues,
            cfg.socks_per_queue(),
        )?;

        let ret = unsafe { xsk::sys::bpf_set_link_xdp_fd(iface_index as i32, prog_fd, 0) };
        if ret != 0 {
            return Err(BpfSetLinkXDPFailed(-ret));
        }

        Ok(XdpProg { iface_index })
    }

    /// Setup the XSK XDP program maps.
    ///
    /// This will initialize the `xsks_map`, `socks_per_queue_map`, `bind_addr_map` and `bind_port_map` maps.
    fn load_xdp_prog_maps(
        obj: *mut xsk::sys::bpf_object,
        bind_addr: Ipv4Addr,
        bind_port: u16,
        queues: &Queues,
        socks_per_queue: usize,
    ) -> Result<()> {
        let xsks_map = Map::new(obj, "xsks_map")?;

        for (socket_idx, socket) in QueuesSocketsRef::from(queues).into_iter().enumerate() {
            xsks_map.set(socket_idx as i32, socket.fd())?;
        }

        Map::new(obj, "socks_per_queue_map")?.set(0, socks_per_queue)?;
        Map::new(obj, "bind_addr_map")?.set(0, u32::from(bind_addr))?;
        Map::new(obj, "bind_port_map")?.set(0, bind_port)?;

        Ok(())
    }

    fn if_nametoindex(interface: String) -> u32 {
        let interface_cstr = CString::new(interface).unwrap();
        unsafe { libc::if_nametoindex(interface_cstr.as_ptr()) }
    }
}

impl Drop for XdpProg {
    fn drop(&mut self) {
        let ret = unsafe { xsk::sys::bpf_set_link_xdp_fd(self.iface_index as i32, -1, 0) };
        if ret < 0 {
            error!("Cannot unload XDP program: errno {}", -ret);
        }
    }
}

/// An eBPF map.
///
/// This object supports just the minimal set of functionalities required to setup the XSK program maps.
struct Map {
    fd:   i32,
    name: String,
}

impl Map {
    fn new(obj: *mut xsk::sys::bpf_object, name: &str) -> Result<Self> {
        let name = name.to_string();
        let name_cstr = CString::new(name.clone()).unwrap();

        let fd = unsafe { xsk::sys::bpf_object__find_map_fd_by_name(obj, name_cstr.as_ptr()) };
        if fd < 0 {
            return Err(MapNotFound(name, -fd));
        }

        Ok(Map { fd, name })
    }

    fn fd(&self) -> i32 {
        self.fd
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn set<T, U>(&self, index: T, value: U) -> Result<()> {
        let ret = unsafe {
            xsk::sys::bpf_map_update_elem(
                self.fd(),
                &index as *const T as *const c_void,
                &value as *const U as *const c_void,
                0,
            )
        };
        if ret < 0 {
            return Err(SetMapFailed(self.name(), -ret));
        }

        Ok(())
    }
}
