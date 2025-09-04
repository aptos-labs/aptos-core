// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble, TidExt};
use std::{any::TypeId, collections::HashMap};

/// Controls how VM session interacts with native extensions. For every session, we consider three
/// kinds of operations:
///   1. `start`: marks the start to some session S1.
///   2. `finish`: marks the end of some session S1. The changed made by S1 are saved, and some new
///      session S2 can now start.
///   3. `abort`: called when some session S2 aborts and needs to roll back to old state (session
///      S1), discarding all changes made by S2.
pub trait NativeExtensionSession {
    /// Called when session is aborted, to roll back the state to some previous session's state,
    /// just before a new session needs to start. If there is no session before
    fn abort(&mut self);

    /// Called on session finish, to save the data modified by the current extension. Called before
    /// a new session is about to start.
    fn finish(&mut self);

    /// Must be called on every session start.
    fn start(&mut self, txn_hash: &[u8; 32], script_hash: &[u8], session_counter: u8);
}

/// Any native extension should implement its version control. This way when a new extension gets
/// added there is a compile-time error when one tries to add it to the native context.
pub trait NativeExtension<'a>: NativeExtensionSession + Tid<'a> {}

impl<'a, T> NativeExtension<'a> for T where T: NativeExtensionSession + Tid<'a> {}

/// Marker for extensions that do not use session model and so there is no need to implement any
/// session operations.
pub trait UnreachableNativeExtensionSession {}

impl<T> NativeExtensionSession for T
where
    T: UnreachableNativeExtensionSession,
{
    fn abort(&mut self) {
        unreachable!("Irrelevant for extension")
    }

    fn finish(&mut self) {
        unreachable!("Irrelevant for extension")
    }

    fn start(&mut self, _txn_hash: &[u8; 32], _script_hash: &[u8], _session_counter: u8) {
        unreachable!("Irrelevant for extension")
    }
}

/// A data type to represent a heterogeneous collection of extensions which are available to
/// native functions. A value to this is passed into the session function execution.
///
/// The implementation uses the crate `better_any` which implements a version of the `Any`
/// type, called `Tid<`a>`, which allows for up to one lifetime parameter. This
/// avoids that extensions need to have `'static` lifetime, which `Any` requires. In order to make a
/// struct suitable to be a 'Tid', use `#[derive(Tid)]` in the struct declaration. (See also
/// tests at the end of this module.)
#[derive(Default)]
pub struct NativeContextExtensions<'a> {
    map: HashMap<TypeId, Box<dyn NativeExtension<'a>>>,
}

impl<'a> NativeContextExtensions<'a> {
    pub fn add<T: NativeExtensionSession + TidAble<'a>>(&mut self, ext: T) {
        assert!(
            self.map.insert(T::id(), Box::new(ext)).is_none(),
            "multiple extensions of the same type not allowed"
        )
    }

    pub fn get<T: NativeExtensionSession + TidAble<'a>>(&self) -> &T {
        self.map
            .get(&T::id())
            .expect("extension unknown")
            .as_ref()
            .downcast_ref::<T>()
            .unwrap()
    }

    pub fn get_mut<T: NativeExtensionSession + TidAble<'a>>(&mut self) -> &mut T {
        self.map
            .get_mut(&T::id())
            .expect("extension unknown")
            .as_mut()
            .downcast_mut::<T>()
            .unwrap()
    }

    pub fn remove<T: NativeExtensionSession + TidAble<'a>>(&mut self) -> T {
        // can't use expect below because it requires `T: Debug`.
        match self
            .map
            .remove(&T::id())
            .expect("extension unknown")
            .downcast_box::<T>()
        {
            Ok(val) => *val,
            Err(_) => panic!("downcast error"),
        }
    }

    pub fn for_each_mut<F>(&mut self, f: F)
    where
        F: Fn(&mut dyn NativeExtensionSession),
    {
        for extension in self.map.values_mut() {
            f(extension.as_mut());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<'a> UnreachableNativeExtensionSession for Ext<'a> {}

    #[derive(Tid)]
    struct Ext<'a> {
        a: &'a mut u64,
    }

    #[test]
    fn non_static_ext() {
        let mut v: u64 = 23;
        let e = Ext { a: &mut v };
        let mut exts = NativeContextExtensions::default();
        exts.add(e);
        *exts.get_mut::<Ext>().a += 1;
        assert_eq!(*exts.get_mut::<Ext>().a, 24);
        *exts.get_mut::<Ext>().a += 1;
        let e1 = exts.remove::<Ext>();
        assert_eq!(*e1.a, 25)
    }
}
