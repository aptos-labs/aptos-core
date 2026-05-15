// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transaction context that wires the [`Loader`] into the interpreter's
//! cross-module dispatch path.

use crate::{read_set::ModuleReadSet, Loader};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    types::InternedTypeList,
    ExecutionContext, FunctionPtr,
};
use mono_move_gas::GasMeter;
use mono_move_runtime::{DescriptorProvider, ObjectDescriptor, ObjectDescriptorTable};

/// Per-transaction execution context. Maintains per-transaction state
/// (gas meter, read-set of loaded modules, object-descriptor table) and
/// serves the interpreter's runtime queries against it.
pub struct TransactionContext<'guard, 'ctx, G: GasMeter> {
    loader: Loader<'guard, 'ctx>,
    read_set: ModuleReadSet<'guard>,
    gas_meter: G,
    /// Object-descriptor table the interpreter reads.
    ///
    /// Invariant: contains every descriptor referenced by any function loaded
    /// so far in this transaction.
    descriptors: ObjectDescriptorTable,
}

impl<'guard, 'ctx, G: GasMeter> TransactionContext<'guard, 'ctx, G> {
    pub fn new(loader: Loader<'guard, 'ctx>, gas_meter: G) -> Self {
        let descriptors = build_descriptor_table(&loader);
        Self {
            loader,
            read_set: ModuleReadSet::new(),
            gas_meter,
            descriptors,
        }
    }

    /// Returns the transaction's read-set.
    pub fn read_set(&self) -> &ModuleReadSet<'guard> {
        &self.read_set
    }

    /// Appends any descriptors the guard has published since this transaction's
    /// table was last synced. Idempotent: a no-op when the guard hasn't grown.
    fn sync_descriptors(&mut self) {
        let records = self.loader.guard().published_vec_descriptors();
        let already_pushed = self.descriptors.len() - reserved_descriptor_count();
        for record in records.into_iter().skip(already_pushed) {
            let offsets = record.elem_ptr_offsets.iter().map(|o| o.0).collect();
            self.descriptors.push(
                ObjectDescriptor::new_vector(record.elem_size, offsets)
                    .expect("guard published an invalid vector descriptor"),
            );
        }
    }
}

impl<'guard, 'ctx, G: GasMeter> ExecutionContext for TransactionContext<'guard, 'ctx, G> {
    fn gas_meter(&mut self) -> &mut impl GasMeter {
        &mut self.gas_meter
    }

    /// Looks up cross-module targets in the read-set, falling back to
    /// the [`Loader`] on cache miss.
    fn load_function(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> anyhow::Result<FunctionPtr> {
        let result = self.loader.load_function(
            &mut self.read_set,
            &mut self.gas_meter,
            module_id,
            name,
            ty_args,
        );
        // A cache miss may have lazily lowered a function and
        // published new vector descriptors to the guard; sync so
        // subsequent `descriptors()` reads see them.
        self.sync_descriptors();
        result
    }
}

impl<'guard, 'ctx, G: GasMeter> DescriptorProvider for TransactionContext<'guard, 'ctx, G> {
    fn descriptors(&self) -> &[ObjectDescriptor] {
        self.descriptors.as_slice()
    }
}

fn build_descriptor_table(loader: &Loader<'_, '_>) -> ObjectDescriptorTable {
    let mut table = ObjectDescriptorTable::new();
    for record in loader.guard().published_vec_descriptors() {
        let offsets = record.elem_ptr_offsets.iter().map(|o| o.0).collect();
        table.push(
            ObjectDescriptor::new_vector(record.elem_size, offsets)
                .expect("guard published an invalid vector descriptor"),
        );
    }
    table
}

/// Count of fixed descriptors `ObjectDescriptorTable::new` installs
/// before any user-published vectors.
fn reserved_descriptor_count() -> usize {
    ObjectDescriptorTable::new().len()
}
