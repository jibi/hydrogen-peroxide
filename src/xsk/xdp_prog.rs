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

use std::{ffi::CString, net::Ipv4Addr, ptr};

use crate::{
    xsk,
    xsk::{Configuration, Error::*, Queues, QueuesSocketsRef, Result},
};

/// An object responsible for managing the lifecycle of an XSK XDP program on a given interface.
pub struct XdpProg {
    iface_index: u32,
    xdp_prog:    *mut xsk::sys::xdp_program,
}

impl XdpProg {
    /// Load a new XSK XDP program on an interface.
    pub fn load(cfg: &Configuration, queues: &Queues) -> Result<Self> {
        let iface_index = Self::if_nametoindex(cfg.interface().to_string());

        let file_cstr = CString::new(cfg.xdp_prog_path().to_string()).unwrap();
        let program_cstr = CString::new("xdp/prog".to_string()).unwrap();

        let xdp_prog = unsafe {
            xsk::sys::xdp_program__open_file(
                file_cstr.as_ptr(),
                program_cstr.as_ptr(),
                ptr::null_mut(),
            )
        };

        if xdp_prog.is_null() {
            return Err(BpfProgLoadFailed(nix::errno::Errno::last_raw()));
        }

        let ret = unsafe {
            xsk::sys::xdp_program__attach(
                xdp_prog,
                iface_index as i32,
                xsk::sys::xdp_attach_mode_XDP_MODE_SKB,
                0,
            )
        };

        if ret != 0 {
            return Err(BpfSetLinkXDPFailed(-ret));
        }

        Self::load_xdp_prog_maps(
            unsafe { xsk::sys::xdp_program__bpf_obj(xdp_prog) },
            cfg.bind_address(),
            cfg.bind_port(),
            queues,
            cfg.socks_per_queue(),
        )?;

        Ok(XdpProg {
            iface_index,
            xdp_prog,
        })
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
        let ret = unsafe {
            xsk::sys::xdp_program__detach(
                self.xdp_prog,
                self.iface_index as i32,
                xsk::sys::xdp_attach_mode_XDP_MODE_SKB,
                0,
            )
        };
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
