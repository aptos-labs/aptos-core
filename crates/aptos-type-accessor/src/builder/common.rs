// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::{MoveModule, MoveType};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub(crate) fn parse_module(
    module: &MoveModule,
) -> (
    // The map of struct to field to field type.
    BTreeMap<Identifier, BTreeMap<Identifier, MoveType>>,
    // Any new Move modules we need to retrieve.
    BTreeSet<ModuleId>,
) {
    let mut structs_info = BTreeMap::new();
    let mut modules_to_retrieve = BTreeSet::new();

    // For each struct in the module look through the types of the fields and
    // determine any more modules we need to look up.
    for struc in &module.structs {
        let mut types_to_resolve = Vec::new();
        let mut types_seen = HashSet::new();

        for field in &struc.fields {
            types_to_resolve.push(field.typ.clone());
            structs_info
                .entry(struc.name.clone().into())
                .or_insert_with(BTreeMap::new)
                .insert(field.name.clone().into(), field.typ.clone());
        }

        // Go through the types recursively until we hit leaf types. As we do so,
        // we add more modules to `modules_to_retrieve`. This way, we can ensure
        // that we look up the types for all modules relevant to this struct.
        while let Some(typ) = types_to_resolve.pop() {
            if types_seen.contains(&typ) {
                continue;
            }
            types_seen.insert(typ.clone());

            // For types that refer to other types, add those to the list of
            // types. This continues until we hit leaves / a cycle.
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

    (structs_info, modules_to_retrieve)
}
