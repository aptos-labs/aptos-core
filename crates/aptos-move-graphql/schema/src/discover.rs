// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_api_types::{MoveModule, MoveStruct, MoveType};
use move_core_types::language_storage::{ModuleId, StructTag};
use std::collections::{BTreeSet, HashSet};

// TODO: The way the builder works now means we end up including more structs in the
// final schema than we actually have to. When we process the top level modules we look
// at what other structs from other modules the structs in the top level modules use.
// We then go through those other modules and do the same. This means we might include
// structs from those other modules that aren't actually used by the top level modules.
// This continues recursively. To fix this the parse functions could return not just
// new modules to fetch, but a list that restricts which structs we work on in those
// modules. This is not necessary for the MVP though, the only downsides of the current
// approach are the schema is sometimes larger than necessary and the odds of a name
// collision (which causes us to use the fully qualified name) are slightly higher.

pub struct MoveStructWithModuleId {
    pub module_id: ModuleId,
    pub struc: MoveStruct,
}

impl MoveStructWithModuleId {
    pub fn struct_tag(&self) -> StructTag {
        StructTag {
            address: *self.module_id.address(),
            module: self.module_id.name().into(),
            name: self.struc.name.clone().into(),
            // TODO: We don't currently handle structs with generic / phantom types.
            type_params: vec![],
        }
    }
}

/// Return MoveStructs for all structs in a Move module and a set of modules we have to
/// fetch based on structs in other modules that the structs in this module depend on.
pub fn discover_structs_for_module(
    module: MoveModule,
) -> Result<(
    // Structs to include in the schema.
    Vec<MoveStructWithModuleId>,
    // Any new Move modules we need to retrieve.
    BTreeSet<ModuleId>,
)> {
    let mut structs = Vec::new();
    let mut modules_to_retrieve = BTreeSet::new();

    let module_id = ModuleId::new(module.address.into(), module.name.into());

    for struc in module.structs.into_iter() {
        let mut types_to_resolve = Vec::new();
        let mut types_seen = HashSet::new();

        // Seed types_to_resolve with the types this struct directly depends on.
        for field in struc.fields.iter() {
            types_to_resolve.push(field.typ.clone());
        }

        let struct_with_module_id = MoveStructWithModuleId {
            module_id: module_id.clone(),
            struc,
        };

        structs.push(struct_with_module_id);

        // Go through the types recursively until we hit leaf types. As we do so,
        // we add more modules to `modules_to_retrieve`. This way, we can ensure
        // that we look up the types for all modules relevant to this struct.
        while let Some(typ) = types_to_resolve.pop() {
            if types_seen.contains(&typ) {
                continue;
            }
            types_seen.insert(typ.clone());

            // For types that refer to other types, add those to the list of types.
            // This continues until we hit leaves / a cycle, which we know to do based
            // on types_seen.
            match typ {
                MoveType::Vector { items: typ } => {
                    types_to_resolve.push(*typ);
                },
                MoveType::Reference {
                    mutable: _,
                    to: typ,
                } => {
                    types_to_resolve.push(*typ);
                },
                MoveType::Struct(struct_tag) => {
                    let module_id =
                        ModuleId::new(struct_tag.address.into(), struct_tag.module.into());
                    modules_to_retrieve.insert(module_id);
                },
                _other => {},
            }
        }
    }

    Ok((structs, modules_to_retrieve))
}
