// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Cases the `raw_memory_access` lint must flag, and ones it must not.
#![allow(dead_code, unused_variables, unused_unsafe)]

use std::ptr::NonNull;

struct Holder {
    data: *mut u8,
    tagged: *const u32,
    // Not flagged: a NonNull is not a raw pointer type, and a reference is not.
    nn: NonNull<u8>,
    r: &'static [u8],
}

fn takes(p: *const u8) -> *mut u64 {
    std::ptr::null_mut()
}

fn written_types(buf: &mut [u8]) {
    let p: *mut u8 = buf.as_mut_ptr();
    let q = buf.as_ptr() as *const u32;
    let n = NonNull::<u8>::dangling();
    // Flagged once: the `*mut u8` is written, and `as_ptr` on a NonNull is
    // a method on a NonNull, not on a raw pointer.
    let back: *mut u8 = n.as_ptr();
}

fn unspelled_pointer(buf: &mut [u8], v: &mut Vec<u8>) {
    // No raw pointer type appears, but the receiver of `add` is one.
    unsafe {
        let x = buf.as_mut_ptr().add(1).read();
        v.as_mut_ptr().add(2).write(0);
        let y = *buf.as_ptr().cast::<u16>();
    }
}

fn free_functions(src: &[u8], dst: &mut [u8]) {
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), 4);
        core::ptr::copy(src.as_ptr(), dst.as_mut_ptr(), 4);
        let s = std::slice::from_raw_parts(src.as_ptr(), 2);
        std::ptr::write_bytes(dst.as_mut_ptr(), 0, 4);
    }
}

fn not_flagged(buf: &mut [u8], nn: NonNull<u8>) {
    // Slices, references, NonNull, and safe byte ops are all fine.
    buf[0] = 1;
    buf.copy_from_slice(&[0; 0][..0]);
    let a = nn.addr();
    let b = buf.as_ptr(); // forming a pointer is not an access; the type is inferred, not written
    let _ = b.is_null(); // ...but calling a method on it is.
}

#[cfg_attr(dylint_lib = "aptos_lints", allow(raw_memory_access))]
fn exempted(p: *mut u8) {
    unsafe { p.add(1).write(0) };
}

fn main() {}
