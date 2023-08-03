// Copyright © Aptos Foundation

use crate::{CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_crypto_derive::{SerializeKey, DeserializeKey};

#[derive(SerializeKey, DeserializeKey)]
struct Test {
    field: u64
}

impl ValidCryptoMaterial for Test {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }
}

impl TryFrom<&[u8]> for Test {
    type Error = CryptoMaterialError;

    fn try_from(_: &[u8]) -> Result<Self, Self::Error> {
        Err(CryptoMaterialError::DeserializationError)
    }
}

#[test]
fn test_deserialize_key_on_generic_struct() {
    #[derive(SerializeKey, DeserializeKey)]
    struct TestGenerics<SubType> {
        field: SubType
    }

    impl<SubType: ValidCryptoMaterial> ValidCryptoMaterial for TestGenerics<SubType> {
        fn to_bytes(&self) -> Vec<u8> {
            self.field.to_bytes()
        }
    }

    impl<SubType: ValidCryptoMaterial> TryFrom<&[u8]> for TestGenerics<SubType> {
        type Error = CryptoMaterialError;

        fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
            SubType::try_from(bytes).map(|field| TestGenerics { field })
        }
    }
}
