use aptos_table_natives::TableChangeSet;
use better_any::{Tid, TidAble};
use std::cell::RefCell;
/// Native context that can be attached to VM `NativeContextExtensions`.
///
/// Note: table resolver is reused for fine-grained storage access.
#[derive(Tid)]
pub struct NativeEvmContext {
    pub(crate) table_change_set: RefCell<TableChangeSet>,
}

impl NativeEvmContext {
    pub fn new() -> Self {
        Self {
            table_change_set: Default::default()
        }
    }

    pub fn into_change_set(self) -> TableChangeSet {
        self.table_change_set.into_inner()
    }    
}
