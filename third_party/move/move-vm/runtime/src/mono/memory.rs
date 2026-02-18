// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::mono::context::{ExecutionContext, StorageId};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use std::{collections::BTreeMap, marker::PhantomData, mem, ops::Range, ptr, slice};

// =========================================================================================
// Memory Views

pub struct MemoryView<'a> {
    ptr: *const u8,
    size: usize,
    _marker: PhantomData<&'a str>,
}

impl<'a> MemoryView<'a> {
    pub fn new(data: &'a [u8]) -> MemoryView<'a> {
        MemoryView {
            ptr: data.as_ptr(),
            size: data.len(),
            _marker: Default::default(),
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn view_as<T: Sized>(&self) -> &T {
        assert_eq!(self.size, mem::size_of::<T>());
        unsafe { &*self.ptr.cast::<T>() }
    }

    #[inline]
    pub fn view_as_mut<T: Sized>(&mut self) -> &mut T {
        assert_eq!(self.size, mem::size_of::<T>());
        unsafe { &mut *self.ptr.cast_mut().cast::<T>() }
    }

    #[inline]
    pub fn view_as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.size) }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn view_as_slice_mut(&self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(mem::transmute::<*const u8, *mut u8>(self.ptr), self.size)
        }
    }

    #[inline]
    pub fn sub_view(&self, offset: usize, size: usize) -> MemoryView<'_> {
        assert!(offset + size < self.size);
        unsafe {
            MemoryView {
                ptr: self.ptr.add(offset),
                size,
                _marker: Default::default(),
            }
        }
    }
}

// =========================================================================================
// Memory Regions

pub struct Region {
    data: Vec<u8>,
}

impl Region {
    pub fn new(initial_capacity: usize) -> Region {
        Region {
            data: Vec::with_capacity(initial_capacity),
        }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn push_uninit(
        &mut self,
        ctx: &dyn ExecutionContext,
        size: usize,
    ) -> PartialVMResult<usize> {
        let cur_size = self.data.len();
        let new_size = cur_size + size;
        if new_size > ctx.stack_bounds().max_size {
            return Err(PartialVMError::new(StatusCode::CALL_STACK_OVERFLOW)
                .with_message("stack exceeded configured size".to_string()));
        }
        self.data.try_reserve(size).map_err(|_| {
            PartialVMError::new(StatusCode::CALL_STACK_OVERFLOW)
                .with_message("stack exceeded available process memory".to_string())
        })?;
        // Since we ensured capacity is sufficient, and the elements are u8 and don't need to be
        // initialized, we can use unsafe set_len to grow the stack.
        unsafe { self.data.set_len(new_size) };
        Ok(cur_size)
    }

    pub fn push_bytes(
        &mut self,
        ctx: &dyn ExecutionContext,
        block: &[u8],
    ) -> PartialVMResult<usize> {
        let size = block.len();
        let offset = self.push_uninit(ctx, size)?;
        unsafe {
            let src = block.as_ptr();
            let dest = self.data.as_mut_ptr().add(offset);
            ptr::copy_nonoverlapping(src, dest, size)
        }
        Ok(offset)
    }

    pub fn pop(&mut self, _ctx: &dyn ExecutionContext, size: usize) {
        unsafe { self.data.set_len(self.data.len() - size) }
    }

    pub fn collapse(&mut self, _ctx: &dyn ExecutionContext, frame_size: usize, result_size: usize) {
        let size = self.data.len();
        unsafe {
            if result_size > 0 {
                let src = self.data.as_ptr().add(size - result_size);
                let dest = self.data.as_mut_ptr().add(size - result_size - frame_size);
                ptr::copy_nonoverlapping(src, dest, result_size)
            }
            self.data.set_len(size - frame_size)
        }
    }

    pub fn slice(&self, range: Range<usize>) -> &[u8] {
        &self.data[range]
    }

    pub fn slice_mut(&mut self, range: Range<usize>) -> &mut [u8] {
        &mut self.data[range]
    }
}

// =========================================================================================
// Memory

pub struct Memory {
    stack: Region,
    heap: Region,
    storage_roots: BTreeMap<StorageId, usize>,
}

#[derive(Clone, Copy)]
pub struct Reference {
    tagged_offset: usize,
}

impl Reference {
    pub fn pack(is_stack: bool, offset: usize) -> Self {
        Reference {
            tagged_offset: offset << 1 | is_stack as usize,
        }
    }

    pub fn local(offset: usize) -> Self {
        Self::pack(true, offset)
    }

    pub fn global(offset: usize) -> Self {
        Self::pack(false, offset)
    }

    #[inline]
    pub fn unpack(self) -> (bool, usize) {
        (self.tagged_offset & 0x1 != 0, self.tagged_offset >> 1)
    }

    #[inline]
    pub fn select_field(self, offset: usize) -> Self {
        let (region, current) = self.unpack();
        Self::pack(region, current + offset)
    }
}

impl Memory {
    pub fn new(ctx: &dyn ExecutionContext) -> Self {
        Self {
            stack: Region::new(ctx.stack_bounds().initial_capacity),
            heap: Region::new(ctx.heap_bounds().initial_capacity),
            storage_roots: BTreeMap::default(),
        }
    }

    // ------------------------------------------------------------------
    // Stack Operations

    pub fn stack_len(&self) -> usize {
        self.stack.size()
    }

    pub fn view(&self, from_top: usize, size: usize) -> MemoryView {
        let start = self.stack_len() - from_top;
        MemoryView::new(self.stack.slice(start..start + size))
    }

    pub fn top_view(&self, size: usize) -> MemoryView<'_> {
        self.view(size, size)
    }

    pub fn push_uninit(
        &mut self,
        ctx: &dyn ExecutionContext,
        size: usize,
    ) -> PartialVMResult<Reference> {
        self.stack.push_uninit(ctx, size).map(Reference::local)
    }

    pub fn push_blob(&mut self, ctx: &dyn ExecutionContext, data: &[u8]) -> PartialVMResult<()> {
        self.stack.push_bytes(ctx, data)?;
        Ok(())
    }

    pub fn push_value<T: Sized>(
        &mut self,
        ctx: &dyn ExecutionContext,
        value: T,
    ) -> PartialVMResult<()> {
        let size = mem::size_of::<T>();
        self.stack.push_uninit(ctx, size)?;
        *self.top_view(size).view_as_mut::<T>() = value;
        Ok(())
    }

    pub fn push_from(
        &mut self,
        ctx: &dyn ExecutionContext,
        from: Reference,
        size: usize,
    ) -> PartialVMResult<()> {
        let new_offset = self.stack.push_uninit(ctx, size)?;
        unsafe {
            let src = match from.unpack() {
                (true, offset) => self.stack.data.as_ptr().add(offset),
                (false, offset) => self.heap.data.as_ptr().add(offset),
            };
            let dest = self.stack.data.as_mut_ptr().add(new_offset);
            ptr::copy_nonoverlapping(src, dest, size);
            Ok(())
        }
    }

    pub fn pop_to(
        &mut self,
        ctx: &dyn ExecutionContext,
        to: Reference,
        size: usize,
    ) -> PartialVMResult<()> {
        let base = self.stack.size() - size;
        unsafe {
            let src = self.stack.data.as_ptr().add(base);
            let dest = match to.unpack() {
                (true, offset) => self.stack.data.as_mut_ptr().add(offset),
                (false, offset) => self.heap.data.as_mut_ptr().add(offset),
            };
            ptr::copy_nonoverlapping(src, dest, size);
        }
        self.stack.pop(ctx, size);
        Ok(())
    }

    pub fn pop_value<T: Sized + Clone>(
        &mut self,
        ctx: &dyn ExecutionContext,
    ) -> PartialVMResult<T> {
        let size = mem::size_of::<T>();
        let res = self.top_view(size).view_as::<T>().clone();
        self.stack.pop(ctx, size);
        Ok(res)
    }

    pub fn collapse(
        &mut self,
        ctx: &dyn ExecutionContext,
        frame_size: usize,
        result_size: usize,
    ) -> PartialVMResult<()> {
        self.stack.collapse(ctx, frame_size, result_size);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Storage Operations

    pub fn borrow_global(
        &mut self,
        ctx: &dyn ExecutionContext,
        id: &StorageId,
    ) -> PartialVMResult<Reference> {
        if let Some(offset) = self.storage_roots.get(id) {
            Ok(Reference::global(*offset))
        } else {
            let data = ctx.fetch_data(id)?;
            // Copy the data to the heap
            let offset = self.heap.push_uninit(ctx, data.len())?;
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    self.stack.data.as_mut_ptr().add(offset),
                    data.len(),
                )
            }
            Ok(Reference::global(offset))
        }
    }
}
