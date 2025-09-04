// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::log::{FrameName, WriteOpType};
use velor_types::{
    access_path::Path,
    state_store::{state_key::StateKey, table::TableHandle},
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
};
use std::fmt::{self, Display};

/// Wrapper to help render the underlying data in human readable formats that are
/// desirable for textual outputs and flamegraphs.
pub(crate) struct Render<'a, T>(pub &'a T);

impl Display for Render<'_, AccountAddress> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let addr_short = self.0.short_str_lossless();
        write!(f, "0x")?;
        if addr_short.len() > 4 {
            write!(f, "{}..", &addr_short[..4])
        } else {
            write!(f, "{}", addr_short)
        }
    }
}

impl Display for Render<'_, ModuleId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", Render(self.0.address()), self.0.name())
    }
}

impl<'a> Display for Render<'a, (&'a ModuleId, &'a IdentStr, &'a [TypeTag])> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", Render(self.0 .0), self.0 .1)?;
        if !self.0 .2.is_empty() {
            write!(
                f,
                "<{}>",
                self.0
                     .2
                    .iter()
                    .map(|ty| ty.to_canonical_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        Ok(())
    }
}

impl Display for FrameName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Script => write!(f, "<script>"),
            Self::Function {
                module_id,
                name: fn_name,
                ty_args,
            } => write!(
                f,
                "{}",
                Render(&(module_id, fn_name.as_ident_str(), ty_args.as_slice())),
            ),
        }
    }
}

impl Display for Render<'_, Path> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Path::Code(module_id) => write!(f, "{}", Render(module_id)),
            Path::Resource(struct_ty) => write!(f, "{}", struct_ty.to_canonical_string()),
            Path::ResourceGroup(struct_ty) => write!(f, "{}", struct_ty.to_canonical_string()),
        }
    }
}

impl Display for Render<'_, TableHandle> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Render(&self.0 .0))
    }
}

pub struct TableKey<'a> {
    pub bytes: &'a [u8],
}

impl Display for TableKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.bytes.len() > 2);
        write!(f, "0x{:02x}{:02x}..", self.bytes[0], self.bytes[1])
    }
}

impl Display for Render<'_, StateKey> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use velor_types::state_store::state_key::inner::StateKeyInner::*;

        match self.0.inner() {
            AccessPath(ap) => {
                write!(f, "{}::{}", Render(&ap.address), Render(&ap.get_path()))
            },
            TableItem { handle, key } => {
                write!(f, "table_item<{},{}>", Render(handle), TableKey {
                    bytes: key
                },)
            },
            Raw(..) => panic!("not supported"),
        }
    }
}

impl Display for Render<'_, WriteOpType> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use WriteOpType::*;

        write!(f, "{}", match self.0 {
            Creation => "create",
            Modification => "modify",
            Deletion => "delete",
        })
    }
}

impl Display for Render<'_, TypeTag> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_canonical_string())
    }
}
