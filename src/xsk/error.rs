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

//! Error handling for XSK operations.

use thiserror::Error;

use std::{ffi::CStr, result};

/// A specialized [`Result`](std::result) type for XSK operations.
///
/// This type is broadly used across `xsk` for any operation which may produce an error.
pub type Result<T> = result::Result<T, Error>;

/// The error type for XSK operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to set rlimit: {}", errno_to_str(.0))]
    SetrlimitFailed(i32),
    #[error("Invalid XSK mode")]
    InvalidXskMode,
    #[error("Invalid XSK config: missing {}", .0)]
    InvalidConfigWithMissingProperty(String),
    #[error("Failed to load BPF program: {}", errno_to_str(.0))]
    BpfProgLoadFailed(i32),
    #[error("Failed to attach XDP program to interface: {}", errno_to_str(.0))]
    BpfSetLinkXDPFailed(i32),
    #[error("Cannot find {} BPF map: {}", .0, errno_to_str(.1))]
    MapNotFound(String, i32),
    #[error("Cannot update {} BPF map: {}", .0, errno_to_str(.1))]
    SetMapFailed(String, i32),
    #[error("Failed to initialise frame allocator: {}", errno_to_str(.0))]
    FrameAllocatorAllocationFailed(i32),
    #[error("Failed to create XSK socket: {}", errno_to_str(.0))]
    XskSocketCreateFailed(i32),
    #[error("Failed to create umem socket: {}", errno_to_str(.0))]
    XskUmemCreateFailed(i32),
    #[error("Failed to reserve descriptors in TX ring")]
    XskFqRingProdReserveFailed,
    #[error("Failed to reserve descriptors in fq ring")]
    XskTxRingProdReserveFailed,
    #[error("poll() on socket fd returned -1: {}", errno_to_str(.0))]
    XskSocketPollFailed(i32),
    #[error("poll() on umem socket returned -1: {}", errno_to_str(.0))]
    XskUmemPollFailed(i32),
    #[error("sendto() returned -1: {}", errno_to_str(.0))]
    XskTxSendtoFailed(i32),
}

fn errno_to_str(err: &i32) -> String {
    let s = unsafe { CStr::from_ptr(libc::strerror(*err)) };
    String::from(s.to_str().unwrap_or_default())
}
