use aptos_table_natives::{TableResolver, TableHandle};
use better_any::{Tid, TidAble};

/// Native context that can be attached to VM `NativeContextExtensions`.
///
/// Note: table resolver is reused for fine-grained storage access.
#[derive(Tid)]
pub struct NativeEvmContext<'a> {
    pub(crate) resolver: &'a dyn TableResolver,
    pub(crate) nonce_table_handle: TableHandle,
    pub(crate) balance_table_handle: TableHandle,
    pub(crate) code_table_handle: TableHandle,
    pub(crate) storage_table_handle: TableHandle,
}

impl<'a> NativeEvmContext<'a> {
    pub fn new(resolver: &'a dyn TableResolver,
               nonce_table_handle: TableHandle,
               balance_table_handle: TableHandle,
               code_table_handle: TableHandle,
               storage_table_handle: TableHandle) -> Self {
        Self {
            resolver,
            nonce_table_handle,
            balance_table_handle,
            code_table_handle,
            storage_table_handle,
        }
    }

    pub fn add_change_set() {
        unimplemented!()
    }
}
