// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::file_format::{Constant, SignatureToken};
use move_core_types::value::{MoveTypeLayout, MoveValue};

fn sig_to_ty(sig: &SignatureToken) -> Option<MoveTypeLayout> {
    match sig {
        SignatureToken::Signer => Some(MoveTypeLayout::Signer),
        SignatureToken::Address => Some(MoveTypeLayout::Address),
        SignatureToken::Bool => Some(MoveTypeLayout::Bool),
        SignatureToken::U8 => Some(MoveTypeLayout::U8),
        SignatureToken::U16 => Some(MoveTypeLayout::U16),
        SignatureToken::U32 => Some(MoveTypeLayout::U32),
        SignatureToken::U64 => Some(MoveTypeLayout::U64),
        SignatureToken::U128 => Some(MoveTypeLayout::U128),
        SignatureToken::U256 => Some(MoveTypeLayout::U256),
        SignatureToken::I8 => Some(MoveTypeLayout::I8),
        SignatureToken::I16 => Some(MoveTypeLayout::I16),
        SignatureToken::I32 => Some(MoveTypeLayout::I32),
        SignatureToken::I64 => Some(MoveTypeLayout::I64),
        SignatureToken::I128 => Some(MoveTypeLayout::I128),
        SignatureToken::I256 => Some(MoveTypeLayout::I256),
        SignatureToken::Vector(v) => Some(MoveTypeLayout::Vector(Box::new(sig_to_ty(v.as_ref())?))),
        SignatureToken::Reference(_)
        | SignatureToken::MutableReference(_)
        | SignatureToken::Struct(_)
        | SignatureToken::Function(..)
        | SignatureToken::TypeParameter(_)
        | SignatureToken::StructInstantiation(_, _) => None,
    }
}

fn construct_ty_for_constant(layout: &MoveTypeLayout) -> Option<SignatureToken> {
    match layout {
        MoveTypeLayout::Address => Some(SignatureToken::Address),
        MoveTypeLayout::Signer => Some(SignatureToken::Signer),
        MoveTypeLayout::U8 => Some(SignatureToken::U8),
        MoveTypeLayout::U16 => Some(SignatureToken::U16),
        MoveTypeLayout::U32 => Some(SignatureToken::U32),
        MoveTypeLayout::U64 => Some(SignatureToken::U64),
        MoveTypeLayout::U128 => Some(SignatureToken::U128),
        MoveTypeLayout::U256 => Some(SignatureToken::U256),
        MoveTypeLayout::I8 => Some(SignatureToken::I8),
        MoveTypeLayout::I16 => Some(SignatureToken::I16),
        MoveTypeLayout::I32 => Some(SignatureToken::I32),
        MoveTypeLayout::I64 => Some(SignatureToken::I64),
        MoveTypeLayout::I128 => Some(SignatureToken::I128),
        MoveTypeLayout::I256 => Some(SignatureToken::I256),
        MoveTypeLayout::Vector(l) => Some(SignatureToken::Vector(Box::new(
            construct_ty_for_constant(l.as_ref())?,
        ))),
        MoveTypeLayout::Struct(_) => None,
        MoveTypeLayout::Function => None,
        MoveTypeLayout::Bool => Some(SignatureToken::Bool),

        // It is not possible to have native layout for constant values.
        MoveTypeLayout::Native(_, _layout) => None,
    }
}

impl Constant {
    pub fn serialize_constant(layout: &MoveTypeLayout, v: &MoveValue) -> Option<Self> {
        Some(Self {
            type_: construct_ty_for_constant(layout)?,
            data: v.simple_serialize()?,
        })
    }

    pub fn deserialize_constant(&self) -> Option<MoveValue> {
        let ty = sig_to_ty(&self.type_)?;
        // Constant types contain no function values, so a constant is never nested deeper
        // than its type, and the depth limit cannot affect verification.
        let (_, type_depth) = self.type_.num_nodes_with_max_depth();
        MoveValue::simple_deserialize_with_max_depth(&self.data, &ty, type_depth as u64).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_format_common::SIGNATURE_TOKEN_DEPTH_MAX;

    #[test]
    fn deep_constant_decodes() {
        // Constants are bounded by the depth of their type, not by the value depth limit.
        let depth = SIGNATURE_TOKEN_DEPTH_MAX - 1;
        let layout = (0..depth).fold(MoveTypeLayout::U8, |l, _| {
            MoveTypeLayout::Vector(Box::new(l))
        });
        let value = (0..depth).fold(MoveValue::U8(1), |v, _| MoveValue::Vector(vec![v]));
        let constant = Constant::serialize_constant(&layout, &value).expect("must serialize");
        assert_eq!(constant.deserialize_constant(), Some(value));
    }
}
