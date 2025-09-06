// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
};
use std::ops::Deref;
use triomphe::Arc;

pub trait WithBytes {
    fn bytes(&self) -> &Bytes;
}

pub trait WithSize {
    fn size_in_bytes(&self) -> usize;
}

impl<T: WithBytes> WithSize for T {
    fn size_in_bytes(&self) -> usize {
        self.bytes().len()
    }
}

pub trait WithHash {
    fn hash(&self) -> &[u8; 32];
}

pub trait WithAddress {
    fn address(&self) -> &AccountAddress;
}

impl WithAddress for ModuleId {
    fn address(&self) -> &AccountAddress {
        self.address()
    }
}

pub trait WithName {
    fn name(&self) -> &IdentStr;
}

impl WithName for ModuleId {
    fn name(&self) -> &IdentStr {
        self.name()
    }
}

/// An entry for the code cache that can have multiple different representations.
pub enum Code<D, V> {
    /// Deserialized code, not yet verified with bytecode verifier.
    Deserialized(Arc<D>),
    /// Fully-verified code.
    Verified(Arc<V>),
}

impl<D, V> Code<D, V>
where
    V: Deref<Target = Arc<D>>,
{
    /// Returns new deserialized code.
    pub fn from_deserialized(deserialized_code: D) -> Self {
        Self::Deserialized(Arc::new(deserialized_code))
    }

    /// Returns new verified code.
    pub fn from_verified(verified_code: V) -> Self {
        Self::Verified(Arc::new(verified_code))
    }

    /// Returns new verified code from [Arc]ed instance.
    pub fn from_arced_verified(verified_code: Arc<V>) -> Self {
        Self::Verified(verified_code)
    }

    /// Returns true if the code is verified.
    pub fn is_verified(&self) -> bool {
        match self {
            Self::Deserialized(_) => false,
            Self::Verified(_) => true,
        }
    }

    /// Returns the deserialized code.
    pub fn deserialized(&self) -> &Arc<D> {
        match self {
            Self::Deserialized(compiled_script) => compiled_script,
            Self::Verified(script) => script.deref(),
        }
    }

    /// Returns the verified code. Panics if the code has not been actually verified.
    pub fn verified(&self) -> &Arc<V> {
        match self {
            Self::Deserialized(_) => {
                unreachable!("This function must be called on verified code only")
            },
            Self::Verified(script) => script,
        }
    }
}

impl<D, V> Clone for Code<D, V> {
    fn clone(&self) -> Self {
        match self {
            Self::Deserialized(code) => Self::Deserialized(code.clone()),
            Self::Verified(code) => Self::Verified(code.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{MockDeserializedCode, MockVerifiedCode};

    #[test]
    fn test_deserialized_code() {
        let deserialized_code = MockDeserializedCode::new(1);
        let code: Code<_, MockVerifiedCode> = Code::from_deserialized(deserialized_code);

        assert!(!code.is_verified());
        assert_eq!(code.deserialized().value(), 1);
        assert!(matches!(code, Code::Deserialized(..)));
    }

    #[test]
    #[should_panic]
    fn test_deserialized_code_panics_if_not_verified() {
        let deserialized_code = MockDeserializedCode::new(1);
        let code: Code<_, MockVerifiedCode> = Code::from_deserialized(deserialized_code);
        code.verified();
    }

    #[test]
    fn test_verified_code() {
        let code = Code::from_verified(MockVerifiedCode::new(1));

        assert!(code.is_verified());
        assert_eq!(code.deserialized().value(), 1);
        assert_eq!(code.verified().value(), 1);
        assert!(matches!(code, Code::Verified(..)));
    }
}
