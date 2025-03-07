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

use crate::net::{Error, Result};

use std::{cmp::max, convert::TryInto, marker::PhantomData, ptr, slice};

enum OffsetOp {
    Set(usize),
    Add(usize),
}

pub struct PacketBufMut<'a> {
    buffer_ptr: *mut u8,
    buffer_len: usize,

    pub packet_offset: usize,
    pub packet_len:    usize,

    phantom: PhantomData<&'a mut [u8]>,
}

impl Default for PacketBufMut<'_> {
    fn default() -> Self {
        PacketBufMut {
            buffer_ptr: ptr::null_mut(),
            buffer_len: 0,

            packet_offset: 0,
            packet_len:    0,

            phantom: PhantomData,
        }
    }
}

impl<'a> PacketBufMut<'a> {
    pub fn from_slice(slice: &'a mut [u8]) -> Self {
        PacketBufMut {
            buffer_ptr: slice.as_mut_ptr(),
            buffer_len: slice.len(),

            packet_offset: 0,
            packet_len:    0,

            phantom: PhantomData,
        }
    }

    pub fn from_raw_parts(buffer_ptr: *mut u8, buffer_len: usize) -> Self {
        PacketBufMut {
            buffer_ptr,
            buffer_len,

            packet_offset: 0,
            packet_len: 0,

            phantom: PhantomData,
        }
    }

    pub fn peek_bytes(&self, n: usize) -> Result<&[u8]> {
        if self.packet_offset + n > self.buffer_len {
            return Err(Error::NotEnoughBytes);
        }

        Ok(unsafe { slice::from_raw_parts(self.buffer_ptr.add(self.packet_offset), n) })
    }

    pub fn get_bytes_mut(&mut self, n: usize) -> Result<&mut [u8]> {
        if self.packet_offset + n > self.buffer_len {
            return Err(Error::NotEnoughBytes);
        }

        let slice =
            unsafe { slice::from_raw_parts_mut(self.buffer_ptr.add(self.packet_offset), n) };

        self.update_offset(OffsetOp::Add(n));

        Ok(slice)
    }

    pub fn get_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.packet_offset + n > self.buffer_len {
            return Err(Error::NotEnoughBytes);
        }

        let slice = unsafe { slice::from_raw_parts(self.buffer_ptr.add(self.packet_offset), n) };

        self.update_offset(OffsetOp::Add(n));

        Ok(slice)
    }

    pub fn peek_u8(&self) -> Result<u8> {
        let bytes: &[u8] = self.peek_bytes(1)?;
        Ok(bytes[0])
    }

    pub fn get_u8(&mut self) -> Result<u8> {
        let bytes: &[u8] = self.get_bytes(1)?;
        Ok(bytes[0])
    }

    pub fn get_be16(&mut self) -> Result<u16> {
        let bytes: [u8; 2] = self.get_bytes(2)?.try_into().unwrap();
        Ok(u16::from_be_bytes(bytes))
    }

    pub fn get_be32(&mut self) -> Result<u32> {
        let bytes: [u8; 4] = self.get_bytes(4)?.try_into().unwrap();
        Ok(u32::from_be_bytes(bytes))
    }

    pub fn get_be64(&mut self) -> Result<u64> {
        let bytes: [u8; 8] = self.get_bytes(8)?.try_into().unwrap();
        Ok(u64::from_be_bytes(bytes))
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.buffer_ptr, self.packet_len) }
    }

    pub fn seek(&mut self, packet_offset: usize) -> Result<()> {
        if packet_offset > self.buffer_len {
            return Err(Error::InvalidSeekPos);
        }

        self.update_offset(OffsetOp::Set(packet_offset));

        Ok(())
    }

    fn update_offset(&mut self, packet_offset: OffsetOp) {
        use OffsetOp::*;

        match packet_offset {
            Set(o) => self.packet_offset = o,
            Add(o) => self.packet_offset += o,
        }
        self.packet_len = max(self.packet_len, self.packet_offset);
    }
}

pub struct PacketBuf<'a> {
    buffer_ptr: *const u8,
    buffer_len: usize,

    pub packet_offset: usize,
    pub packet_len:    usize,

    phantom: PhantomData<&'a [u8]>,
}

impl Default for PacketBuf<'_> {
    fn default() -> Self {
        PacketBuf {
            buffer_ptr: ptr::null_mut(),
            buffer_len: 0,

            packet_offset: 0,
            packet_len:    0,

            phantom: PhantomData,
        }
    }
}

impl<'a> PacketBuf<'a> {
    pub fn from_slice(slice: &'a [u8]) -> Self {
        PacketBuf {
            buffer_ptr: slice.as_ptr(),
            buffer_len: slice.len(),

            packet_offset: 0,
            packet_len:    0,

            phantom: PhantomData,
        }
    }

    pub fn from_raw_parts(buffer_ptr: *mut u8, buffer_len: usize) -> Self {
        PacketBuf {
            buffer_ptr,
            buffer_len,

            packet_offset: 0,
            packet_len: 0,

            phantom: PhantomData,
        }
    }

    pub fn peek_bytes(&self, n: usize) -> Result<&[u8]> {
        if self.packet_offset + n > self.buffer_len {
            return Err(Error::NotEnoughBytes);
        }

        Ok(unsafe { slice::from_raw_parts(self.buffer_ptr.add(self.packet_offset), n) })
    }

    pub fn get_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.packet_offset + n > self.buffer_len {
            return Err(Error::NotEnoughBytes);
        }

        let slice = unsafe { slice::from_raw_parts(self.buffer_ptr.add(self.packet_offset), n) };

        self.update_offset(OffsetOp::Add(n));

        Ok(slice)
    }

    pub fn peek_u8(&self) -> Result<u8> {
        let bytes: &[u8] = self.peek_bytes(1)?;
        Ok(bytes[0])
    }

    pub fn get_u8(&mut self) -> Result<u8> {
        let bytes: &[u8] = self.get_bytes(1)?;
        Ok(bytes[0])
    }

    pub fn get_be16(&mut self) -> Result<u16> {
        let bytes: [u8; 2] = self.get_bytes(2)?.try_into().unwrap();
        Ok(u16::from_be_bytes(bytes))
    }

    pub fn get_be32(&mut self) -> Result<u32> {
        let bytes: [u8; 4] = self.get_bytes(4)?.try_into().unwrap();
        Ok(u32::from_be_bytes(bytes))
    }

    pub fn get_be64(&mut self) -> Result<u64> {
        let bytes: [u8; 8] = self.get_bytes(8)?.try_into().unwrap();
        Ok(u64::from_be_bytes(bytes))
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.buffer_ptr, self.packet_len) }
    }

    pub fn seek(&mut self, packet_offset: usize) -> Result<()> {
        if packet_offset > self.buffer_len {
            return Err(Error::InvalidSeekPos);
        }

        self.update_offset(OffsetOp::Set(packet_offset));

        Ok(())
    }

    fn update_offset(&mut self, packet_offset: OffsetOp) {
        use OffsetOp::*;

        match packet_offset {
            Set(o) => self.packet_offset = o,
            Add(o) => self.packet_offset += o,
        }
        self.packet_len = max(self.packet_len, self.packet_offset);
    }
}
