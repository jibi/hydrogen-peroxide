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

use crate::{net::app::AppAllocator, xsk};

/// Configuration builder for a App object.
#[derive(Default)]
pub struct Configuration {
    app_allocator: Option<Box<AppAllocator>>,
    xsk_handle:    Option<xsk::net::Handle>,
}


impl Configuration {
    /// Set the AppAllocator callback.
    pub fn set_app_allocator(&mut self, app_allocator: Box<AppAllocator>) -> &mut Self {
        self.app_allocator = Some(app_allocator);
        self
    }

    /// Get the AppAllocator callback.
    pub fn app_allocator(&self) -> &AppAllocator {
        self.app_allocator.as_ref().unwrap()
    }

    /// Set the XSK handle.
    pub fn set_xsk_handle(&mut self, xsk_handle: xsk::net::Handle) -> &mut Self {
        self.xsk_handle = Some(xsk_handle);
        self
    }

    /// Get the XSK handle.
    pub fn take_xsk_handle(&mut self) -> xsk::net::Handle {
        self.xsk_handle.take().unwrap()
    }
}
