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

use nix::ifaddrs::getifaddrs;

pub fn ntohs(x: u16) -> u16 {
    u16::from_be(x)
}

pub fn ntohl(x: u32) -> u32 {
    u32::from_be(x)
}

pub fn htons(x: u16) -> u16 {
    x.to_be()
}

pub fn htonl(x: u32) -> u32 {
    x.to_be()
}

pub fn mac_to_string(addr: [u8; 6]) -> String {
    addr.iter()
        .map(|x| format!("{:02X}", x))
        .collect::<Vec<String>>()
        .join(":")
}

pub fn get_phy_mac_addr(iface: &str) -> Option<[u8; 6]> {
    for addr in getifaddrs().unwrap().filter(|v| v.interface_name == iface) {
        if let Some(nix::sys::socket::SockAddr::Link(address)) = addr.address {
            return Some(address.addr());
        }
    }

    None
}
