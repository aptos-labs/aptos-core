// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! A simple foolproof C-string cache.
//!
//! It's easy to create dangling pointers when creating C strings. The
//! `SafeCStr` trait offers a simple conversion method while not allowing
//! dangling pointers. It uses a thread-local cache to just store the referents
//! forever. The wasted memory will likely never matter for this compiler, and
//! is easy to refactor into something more robust if it ever does matter.

use std::{cell::RefCell, collections::HashMap, ffi::CString, thread_local};

pub trait SafeCStr {
    fn cstr(&self) -> *const libc::c_char;
}

impl<T> SafeCStr for T
where
    T: AsRef<str>,
{
    fn cstr(&self) -> *const libc::c_char {
        cstr(self.as_ref())
    }
}

fn cstr(s: &str) -> *const libc::c_char {
    thread_local! {
        static MAP: RefCell<HashMap<String, CString>> = RefCell::new(HashMap::new());
    }

    MAP.with(|map| {
        let mut map = map.borrow_mut();
        if let Some(val) = map.get(s) {
            val.as_ptr()
        } else {
            // Allocate space for null byte, applied by CString::new
            let mut new_str = String::with_capacity(s.len() + 1);
            new_str.push_str(s);
            let new_cstr = CString::new(new_str).expect("interior nul byte");
            let ptr = new_cstr.as_ptr();
            map.insert(s.to_owned(), new_cstr);
            ptr
        }
    })
}
