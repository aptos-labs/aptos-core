// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::ptr::NonNull;

#[derive(Debug)]
pub(crate) enum Ref<T> {
    Strong(Box<T>),
    Weak(NonNull<T>),
}

impl<T> Ref<T> {
    pub fn get_raw(&self) -> NonNull<T> {
        match self {
            Self::Strong(r#box) => NonNull::from_ref(r#box),
            Self::Weak(ptr) => *ptr,
        }
    }

    pub fn is_strong(&self) -> bool {
        matches!(self, Self::Strong(_))
    }

    pub fn from_raw(ptr: NonNull<T>) -> Self {
        Self::Weak(ptr)
    }
}

// impl<T> Clone for Ref<T> {
//     fn clone(&self) -> Self {
//         match self {
//             Self::Strong(arc) => Self::Strong(arc.clone()),
//             Self::Weak(weak) => Self::Weak(weak.clone()),
//         }
//     }
// }
// 
// impl<T> Ref<T> {
//     pub fn try_get_strong(&self) -> Option<Arc<T>> {
//         match self {
//             Self::Strong(arc) => Some(arc.clone()),
//             Self::Weak(weak) => weak.upgrade(),
//         }
//     }
// }
