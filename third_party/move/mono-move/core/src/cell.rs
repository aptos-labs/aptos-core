// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Anchored borrows of `UnsafeCell` fields: borrows whose lifetime is tied
//! to a local variable, the anchor.
//!
//! A reference made from `UnsafeCell::get()` comes through a raw pointer, so
//! the compiler does not know that it should not outlive the function that
//! made it. [`borrow_cell!`](crate::borrow_cell) ties it to a local
//! [`Anchor`], and the compiler's ordinary rule then applies: a reference to
//! a local cannot be returned, stored through a parameter, or kept by a
//! callee.
//!
//! The anchor only stops a borrow from leaving its function. The
//! `unsafe_cell_reborrow` lint checks that two borrows of the same field do
//! not overlap, and that every borrow of such a field is anchored; see
//! `devtools/lints/docs/unsafe_cell_reborrow.md`.

/// A local that a cell borrow is tied to. Holds nothing.
pub struct Anchor;

/// A reference whose lifetime can be shortened to `'s`: `&T` or `&mut T`.
pub trait CellBorrow<'s> {
    type Anchored;

    fn anchored(self) -> Self::Anchored;
}

impl<'s, 'l: 's, T: ?Sized> CellBorrow<'s> for &'l T {
    type Anchored = &'s T;

    #[inline(always)]
    fn anchored(self) -> &'s T {
        self
    }
}

impl<'s, 'l: 's, T: ?Sized> CellBorrow<'s> for &'l mut T {
    type Anchored = &'s mut T;

    #[inline(always)]
    fn anchored(self) -> &'s mut T {
        self
    }
}

/// Shortens `borrow` to the lifetime of `anchor`. Does nothing at run time.
///
/// The anchor is borrowed exclusively, so two borrows made from the same
/// anchor cannot be live together.
#[inline(always)]
pub fn anchor_borrow<'s, B: CellBorrow<'s>>(_anchor: &'s mut Anchor, borrow: B) -> B::Anchored {
    borrow.anchored()
}

/// Borrows an `UnsafeCell` field, tied to a fresh local [`Anchor`]:
///
/// ```ignore
/// // SAFETY: ...
/// borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
/// ```
///
/// `heap` then cannot outlive the enclosing block. The `unsafe` expression is
/// written at the call site, so that it stays visible there and keeps its
/// `SAFETY` comment.
#[macro_export]
macro_rules! borrow_cell {
    (let $name:ident = $borrow:expr) => {
        let mut anchor = $crate::cell::Anchor;
        let $name = $crate::cell::anchor_borrow(&mut anchor, $borrow);
    };
}
