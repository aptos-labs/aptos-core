// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// @generated
impl serde::Serialize for AbstractionSignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.function_info.is_empty() {
            len += 1;
        }
        if !self.signature.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.AbstractionSignature", len)?;
        if !self.function_info.is_empty() {
            struct_ser.serialize_field("functionInfo", &self.function_info)?;
        }
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AbstractionSignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "function_info",
            "functionInfo",
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            FunctionInfo,
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "functionInfo" | "function_info" => Ok(GeneratedField::FunctionInfo),
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AbstractionSignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.AbstractionSignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<AbstractionSignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut function_info__ = None;
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::FunctionInfo => {
                            if function_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("functionInfo"));
                            }
                            function_info__ = Some(map.next_value()?);
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(AbstractionSignature {
                    function_info: function_info__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.AbstractionSignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AccountSignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if self.signature.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.AccountSignature", len)?;
        if self.r#type != 0 {
            let v = account_signature::Type::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if let Some(v) = self.signature.as_ref() {
            match v {
                account_signature::Signature::Ed25519(v) => {
                    struct_ser.serialize_field("ed25519", v)?;
                }
                account_signature::Signature::MultiEd25519(v) => {
                    struct_ser.serialize_field("multiEd25519", v)?;
                }
                account_signature::Signature::SingleKeySignature(v) => {
                    struct_ser.serialize_field("singleKeySignature", v)?;
                }
                account_signature::Signature::MultiKeySignature(v) => {
                    struct_ser.serialize_field("multiKeySignature", v)?;
                }
                account_signature::Signature::Abstraction(v) => {
                    struct_ser.serialize_field("abstraction", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AccountSignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "ed25519",
            "multi_ed25519",
            "multiEd25519",
            "single_key_signature",
            "singleKeySignature",
            "multi_key_signature",
            "multiKeySignature",
            "abstraction",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Ed25519,
            MultiEd25519,
            SingleKeySignature,
            MultiKeySignature,
            Abstraction,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "ed25519" => Ok(GeneratedField::Ed25519),
                            "multiEd25519" | "multi_ed25519" => Ok(GeneratedField::MultiEd25519),
                            "singleKeySignature" | "single_key_signature" => Ok(GeneratedField::SingleKeySignature),
                            "multiKeySignature" | "multi_key_signature" => Ok(GeneratedField::MultiKeySignature),
                            "abstraction" => Ok(GeneratedField::Abstraction),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AccountSignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.AccountSignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<AccountSignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<account_signature::Type>()? as i32);
                        }
                        GeneratedField::Ed25519 => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ed25519"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(account_signature::Signature::Ed25519)
;
                        }
                        GeneratedField::MultiEd25519 => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiEd25519"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(account_signature::Signature::MultiEd25519)
;
                        }
                        GeneratedField::SingleKeySignature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("singleKeySignature"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(account_signature::Signature::SingleKeySignature)
;
                        }
                        GeneratedField::MultiKeySignature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiKeySignature"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(account_signature::Signature::MultiKeySignature)
;
                        }
                        GeneratedField::Abstraction => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("abstraction"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(account_signature::Signature::Abstraction)
;
                        }
                    }
                }
                Ok(AccountSignature {
                    r#type: r#type__.unwrap_or_default(),
                    signature: signature__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.AccountSignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for account_signature::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::Ed25519 => "TYPE_ED25519",
            Self::MultiEd25519 => "TYPE_MULTI_ED25519",
            Self::SingleKey => "TYPE_SINGLE_KEY",
            Self::MultiKey => "TYPE_MULTI_KEY",
            Self::Abstraction => "TYPE_ABSTRACTION",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for account_signature::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "TYPE_ED25519",
            "TYPE_MULTI_ED25519",
            "TYPE_SINGLE_KEY",
            "TYPE_MULTI_KEY",
            "TYPE_ABSTRACTION",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = account_signature::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(account_signature::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(account_signature::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(account_signature::Type::Unspecified),
                    "TYPE_ED25519" => Ok(account_signature::Type::Ed25519),
                    "TYPE_MULTI_ED25519" => Ok(account_signature::Type::MultiEd25519),
                    "TYPE_SINGLE_KEY" => Ok(account_signature::Type::SingleKey),
                    "TYPE_MULTI_KEY" => Ok(account_signature::Type::MultiKey),
                    "TYPE_ABSTRACTION" => Ok(account_signature::Type::Abstraction),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for AnyPublicKey {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if !self.public_key.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.AnyPublicKey", len)?;
        if self.r#type != 0 {
            let v = any_public_key::Type::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if !self.public_key.is_empty() {
            struct_ser.serialize_field("publicKey", pbjson::private::base64::encode(&self.public_key).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AnyPublicKey {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "public_key",
            "publicKey",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            PublicKey,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "publicKey" | "public_key" => Ok(GeneratedField::PublicKey),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AnyPublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.AnyPublicKey")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<AnyPublicKey, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut public_key__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<any_public_key::Type>()? as i32);
                        }
                        GeneratedField::PublicKey => {
                            if public_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKey"));
                            }
                            public_key__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(AnyPublicKey {
                    r#type: r#type__.unwrap_or_default(),
                    public_key: public_key__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.AnyPublicKey", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for any_public_key::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::Ed25519 => "TYPE_ED25519",
            Self::Secp256k1Ecdsa => "TYPE_SECP256K1_ECDSA",
            Self::Secp256r1Ecdsa => "TYPE_SECP256R1_ECDSA",
            Self::Keyless => "TYPE_KEYLESS",
            Self::FederatedKeyless => "TYPE_FEDERATED_KEYLESS",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for any_public_key::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "TYPE_ED25519",
            "TYPE_SECP256K1_ECDSA",
            "TYPE_SECP256R1_ECDSA",
            "TYPE_KEYLESS",
            "TYPE_FEDERATED_KEYLESS",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = any_public_key::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(any_public_key::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(any_public_key::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(any_public_key::Type::Unspecified),
                    "TYPE_ED25519" => Ok(any_public_key::Type::Ed25519),
                    "TYPE_SECP256K1_ECDSA" => Ok(any_public_key::Type::Secp256k1Ecdsa),
                    "TYPE_SECP256R1_ECDSA" => Ok(any_public_key::Type::Secp256r1Ecdsa),
                    "TYPE_KEYLESS" => Ok(any_public_key::Type::Keyless),
                    "TYPE_FEDERATED_KEYLESS" => Ok(any_public_key::Type::FederatedKeyless),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for AnySignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if !self.signature.is_empty() {
            len += 1;
        }
        if self.signature_variant.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.AnySignature", len)?;
        if self.r#type != 0 {
            let v = any_signature::Type::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        if let Some(v) = self.signature_variant.as_ref() {
            match v {
                any_signature::SignatureVariant::Ed25519(v) => {
                    struct_ser.serialize_field("ed25519", v)?;
                }
                any_signature::SignatureVariant::Secp256k1Ecdsa(v) => {
                    struct_ser.serialize_field("secp256k1Ecdsa", v)?;
                }
                any_signature::SignatureVariant::Webauthn(v) => {
                    struct_ser.serialize_field("webauthn", v)?;
                }
                any_signature::SignatureVariant::Keyless(v) => {
                    struct_ser.serialize_field("keyless", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AnySignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "signature",
            "ed25519",
            "secp256k1_ecdsa",
            "secp256k1Ecdsa",
            "webauthn",
            "keyless",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Signature,
            Ed25519,
            Secp256k1Ecdsa,
            Webauthn,
            Keyless,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "signature" => Ok(GeneratedField::Signature),
                            "ed25519" => Ok(GeneratedField::Ed25519),
                            "secp256k1Ecdsa" | "secp256k1_ecdsa" => Ok(GeneratedField::Secp256k1Ecdsa),
                            "webauthn" => Ok(GeneratedField::Webauthn),
                            "keyless" => Ok(GeneratedField::Keyless),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AnySignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.AnySignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<AnySignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut signature__ = None;
                let mut signature_variant__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<any_signature::Type>()? as i32);
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Ed25519 => {
                            if signature_variant__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ed25519"));
                            }
                            signature_variant__ = map.next_value::<::std::option::Option<_>>()?.map(any_signature::SignatureVariant::Ed25519)
;
                        }
                        GeneratedField::Secp256k1Ecdsa => {
                            if signature_variant__.is_some() {
                                return Err(serde::de::Error::duplicate_field("secp256k1Ecdsa"));
                            }
                            signature_variant__ = map.next_value::<::std::option::Option<_>>()?.map(any_signature::SignatureVariant::Secp256k1Ecdsa)
;
                        }
                        GeneratedField::Webauthn => {
                            if signature_variant__.is_some() {
                                return Err(serde::de::Error::duplicate_field("webauthn"));
                            }
                            signature_variant__ = map.next_value::<::std::option::Option<_>>()?.map(any_signature::SignatureVariant::Webauthn)
;
                        }
                        GeneratedField::Keyless => {
                            if signature_variant__.is_some() {
                                return Err(serde::de::Error::duplicate_field("keyless"));
                            }
                            signature_variant__ = map.next_value::<::std::option::Option<_>>()?.map(any_signature::SignatureVariant::Keyless)
;
                        }
                    }
                }
                Ok(AnySignature {
                    r#type: r#type__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                    signature_variant: signature_variant__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.AnySignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for any_signature::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::Ed25519 => "TYPE_ED25519",
            Self::Secp256k1Ecdsa => "TYPE_SECP256K1_ECDSA",
            Self::Webauthn => "TYPE_WEBAUTHN",
            Self::Keyless => "TYPE_KEYLESS",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for any_signature::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "TYPE_ED25519",
            "TYPE_SECP256K1_ECDSA",
            "TYPE_WEBAUTHN",
            "TYPE_KEYLESS",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = any_signature::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(any_signature::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(any_signature::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(any_signature::Type::Unspecified),
                    "TYPE_ED25519" => Ok(any_signature::Type::Ed25519),
                    "TYPE_SECP256K1_ECDSA" => Ok(any_signature::Type::Secp256k1Ecdsa),
                    "TYPE_WEBAUTHN" => Ok(any_signature::Type::Webauthn),
                    "TYPE_KEYLESS" => Ok(any_signature::Type::Keyless),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Block {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.height != 0 {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.chain_id != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.Block", len)?;
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if self.height != 0 {
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        if self.chain_id != 0 {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Block {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "timestamp",
            "height",
            "transactions",
            "chain_id",
            "chainId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Timestamp,
            Height,
            Transactions,
            ChainId,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "height" => Ok(GeneratedField::Height),
                            "transactions" => Ok(GeneratedField::Transactions),
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Block;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.Block")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Block, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut timestamp__ = None;
                let mut height__ = None;
                let mut transactions__ = None;
                let mut chain_id__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map.next_value()?;
                        }
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ = Some(map.next_value()?);
                        }
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Block {
                    timestamp: timestamp__,
                    height: height__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                    chain_id: chain_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.Block", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockEndInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.block_gas_limit_reached {
            len += 1;
        }
        if self.block_output_limit_reached {
            len += 1;
        }
        if self.block_effective_block_gas_units != 0 {
            len += 1;
        }
        if self.block_approx_output_size != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.BlockEndInfo", len)?;
        if self.block_gas_limit_reached {
            struct_ser.serialize_field("blockGasLimitReached", &self.block_gas_limit_reached)?;
        }
        if self.block_output_limit_reached {
            struct_ser.serialize_field("blockOutputLimitReached", &self.block_output_limit_reached)?;
        }
        if self.block_effective_block_gas_units != 0 {
            struct_ser.serialize_field("blockEffectiveBlockGasUnits", ToString::to_string(&self.block_effective_block_gas_units).as_str())?;
        }
        if self.block_approx_output_size != 0 {
            struct_ser.serialize_field("blockApproxOutputSize", ToString::to_string(&self.block_approx_output_size).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockEndInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block_gas_limit_reached",
            "blockGasLimitReached",
            "block_output_limit_reached",
            "blockOutputLimitReached",
            "block_effective_block_gas_units",
            "blockEffectiveBlockGasUnits",
            "block_approx_output_size",
            "blockApproxOutputSize",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockGasLimitReached,
            BlockOutputLimitReached,
            BlockEffectiveBlockGasUnits,
            BlockApproxOutputSize,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "blockGasLimitReached" | "block_gas_limit_reached" => Ok(GeneratedField::BlockGasLimitReached),
                            "blockOutputLimitReached" | "block_output_limit_reached" => Ok(GeneratedField::BlockOutputLimitReached),
                            "blockEffectiveBlockGasUnits" | "block_effective_block_gas_units" => Ok(GeneratedField::BlockEffectiveBlockGasUnits),
                            "blockApproxOutputSize" | "block_approx_output_size" => Ok(GeneratedField::BlockApproxOutputSize),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockEndInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.BlockEndInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<BlockEndInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block_gas_limit_reached__ = None;
                let mut block_output_limit_reached__ = None;
                let mut block_effective_block_gas_units__ = None;
                let mut block_approx_output_size__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::BlockGasLimitReached => {
                            if block_gas_limit_reached__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockGasLimitReached"));
                            }
                            block_gas_limit_reached__ = Some(map.next_value()?);
                        }
                        GeneratedField::BlockOutputLimitReached => {
                            if block_output_limit_reached__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockOutputLimitReached"));
                            }
                            block_output_limit_reached__ = Some(map.next_value()?);
                        }
                        GeneratedField::BlockEffectiveBlockGasUnits => {
                            if block_effective_block_gas_units__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockEffectiveBlockGasUnits"));
                            }
                            block_effective_block_gas_units__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BlockApproxOutputSize => {
                            if block_approx_output_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockApproxOutputSize"));
                            }
                            block_approx_output_size__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(BlockEndInfo {
                    block_gas_limit_reached: block_gas_limit_reached__.unwrap_or_default(),
                    block_output_limit_reached: block_output_limit_reached__.unwrap_or_default(),
                    block_effective_block_gas_units: block_effective_block_gas_units__.unwrap_or_default(),
                    block_approx_output_size: block_approx_output_size__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.BlockEndInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockEpilogueTransaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.block_end_info.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.BlockEpilogueTransaction", len)?;
        if let Some(v) = self.block_end_info.as_ref() {
            struct_ser.serialize_field("blockEndInfo", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockEpilogueTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block_end_info",
            "blockEndInfo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockEndInfo,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "blockEndInfo" | "block_end_info" => Ok(GeneratedField::BlockEndInfo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockEpilogueTransaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.BlockEpilogueTransaction")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<BlockEpilogueTransaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block_end_info__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::BlockEndInfo => {
                            if block_end_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockEndInfo"));
                            }
                            block_end_info__ = map.next_value()?;
                        }
                    }
                }
                Ok(BlockEpilogueTransaction {
                    block_end_info: block_end_info__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.BlockEpilogueTransaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockMetadataTransaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if self.round != 0 {
            len += 1;
        }
        if !self.events.is_empty() {
            len += 1;
        }
        if !self.previous_block_votes_bitvec.is_empty() {
            len += 1;
        }
        if !self.proposer.is_empty() {
            len += 1;
        }
        if !self.failed_proposer_indices.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.BlockMetadataTransaction", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if self.round != 0 {
            struct_ser.serialize_field("round", ToString::to_string(&self.round).as_str())?;
        }
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        if !self.previous_block_votes_bitvec.is_empty() {
            struct_ser.serialize_field("previousBlockVotesBitvec", pbjson::private::base64::encode(&self.previous_block_votes_bitvec).as_str())?;
        }
        if !self.proposer.is_empty() {
            struct_ser.serialize_field("proposer", &self.proposer)?;
        }
        if !self.failed_proposer_indices.is_empty() {
            struct_ser.serialize_field("failedProposerIndices", &self.failed_proposer_indices)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockMetadataTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "round",
            "events",
            "previous_block_votes_bitvec",
            "previousBlockVotesBitvec",
            "proposer",
            "failed_proposer_indices",
            "failedProposerIndices",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Round,
            Events,
            PreviousBlockVotesBitvec,
            Proposer,
            FailedProposerIndices,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "round" => Ok(GeneratedField::Round),
                            "events" => Ok(GeneratedField::Events),
                            "previousBlockVotesBitvec" | "previous_block_votes_bitvec" => Ok(GeneratedField::PreviousBlockVotesBitvec),
                            "proposer" => Ok(GeneratedField::Proposer),
                            "failedProposerIndices" | "failed_proposer_indices" => Ok(GeneratedField::FailedProposerIndices),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockMetadataTransaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.BlockMetadataTransaction")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<BlockMetadataTransaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut round__ = None;
                let mut events__ = None;
                let mut previous_block_votes_bitvec__ = None;
                let mut proposer__ = None;
                let mut failed_proposer_indices__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map.next_value()?);
                        }
                        GeneratedField::Round => {
                            if round__.is_some() {
                                return Err(serde::de::Error::duplicate_field("round"));
                            }
                            round__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                        GeneratedField::PreviousBlockVotesBitvec => {
                            if previous_block_votes_bitvec__.is_some() {
                                return Err(serde::de::Error::duplicate_field("previousBlockVotesBitvec"));
                            }
                            previous_block_votes_bitvec__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Proposer => {
                            if proposer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proposer"));
                            }
                            proposer__ = Some(map.next_value()?);
                        }
                        GeneratedField::FailedProposerIndices => {
                            if failed_proposer_indices__.is_some() {
                                return Err(serde::de::Error::duplicate_field("failedProposerIndices"));
                            }
                            failed_proposer_indices__ =
                                Some(map.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(BlockMetadataTransaction {
                    id: id__.unwrap_or_default(),
                    round: round__.unwrap_or_default(),
                    events: events__.unwrap_or_default(),
                    previous_block_votes_bitvec: previous_block_votes_bitvec__.unwrap_or_default(),
                    proposer: proposer__.unwrap_or_default(),
                    failed_proposer_indices: failed_proposer_indices__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.BlockMetadataTransaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteModule {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.address.is_empty() {
            len += 1;
        }
        if !self.state_key_hash.is_empty() {
            len += 1;
        }
        if self.module.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.DeleteModule", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field("stateKeyHash", pbjson::private::base64::encode(&self.state_key_hash).as_str())?;
        }
        if let Some(v) = self.module.as_ref() {
            struct_ser.serialize_field("module", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteModule {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "state_key_hash",
            "stateKeyHash",
            "module",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            StateKeyHash,
            Module,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "stateKeyHash" | "state_key_hash" => Ok(GeneratedField::StateKeyHash),
                            "module" => Ok(GeneratedField::Module),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteModule;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.DeleteModule")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteModule, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut state_key_hash__ = None;
                let mut module__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        }
                        GeneratedField::StateKeyHash => {
                            if state_key_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateKeyHash"));
                            }
                            state_key_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Module => {
                            if module__.is_some() {
                                return Err(serde::de::Error::duplicate_field("module"));
                            }
                            module__ = map.next_value()?;
                        }
                    }
                }
                Ok(DeleteModule {
                    address: address__.unwrap_or_default(),
                    state_key_hash: state_key_hash__.unwrap_or_default(),
                    module: module__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.DeleteModule", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteResource {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.address.is_empty() {
            len += 1;
        }
        if !self.state_key_hash.is_empty() {
            len += 1;
        }
        if self.r#type.is_some() {
            len += 1;
        }
        if !self.type_str.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.DeleteResource", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field("stateKeyHash", pbjson::private::base64::encode(&self.state_key_hash).as_str())?;
        }
        if let Some(v) = self.r#type.as_ref() {
            struct_ser.serialize_field("type", v)?;
        }
        if !self.type_str.is_empty() {
            struct_ser.serialize_field("typeStr", &self.type_str)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteResource {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "state_key_hash",
            "stateKeyHash",
            "type",
            "type_str",
            "typeStr",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            StateKeyHash,
            Type,
            TypeStr,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "stateKeyHash" | "state_key_hash" => Ok(GeneratedField::StateKeyHash),
                            "type" => Ok(GeneratedField::Type),
                            "typeStr" | "type_str" => Ok(GeneratedField::TypeStr),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteResource;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.DeleteResource")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteResource, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut state_key_hash__ = None;
                let mut r#type__ = None;
                let mut type_str__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        }
                        GeneratedField::StateKeyHash => {
                            if state_key_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateKeyHash"));
                            }
                            state_key_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = map.next_value()?;
                        }
                        GeneratedField::TypeStr => {
                            if type_str__.is_some() {
                                return Err(serde::de::Error::duplicate_field("typeStr"));
                            }
                            type_str__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DeleteResource {
                    address: address__.unwrap_or_default(),
                    state_key_hash: state_key_hash__.unwrap_or_default(),
                    r#type: r#type__,
                    type_str: type_str__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.DeleteResource", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteTableData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        if !self.key_type.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.DeleteTableData", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if !self.key_type.is_empty() {
            struct_ser.serialize_field("keyType", &self.key_type)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteTableData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "key_type",
            "keyType",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            KeyType,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "keyType" | "key_type" => Ok(GeneratedField::KeyType),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteTableData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.DeleteTableData")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteTableData, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut key_type__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map.next_value()?);
                        }
                        GeneratedField::KeyType => {
                            if key_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("keyType"));
                            }
                            key_type__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DeleteTableData {
                    key: key__.unwrap_or_default(),
                    key_type: key_type__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.DeleteTableData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteTableItem {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.state_key_hash.is_empty() {
            len += 1;
        }
        if !self.handle.is_empty() {
            len += 1;
        }
        if !self.key.is_empty() {
            len += 1;
        }
        if self.data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.DeleteTableItem", len)?;
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field("stateKeyHash", pbjson::private::base64::encode(&self.state_key_hash).as_str())?;
        }
        if !self.handle.is_empty() {
            struct_ser.serialize_field("handle", &self.handle)?;
        }
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteTableItem {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "state_key_hash",
            "stateKeyHash",
            "handle",
            "key",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StateKeyHash,
            Handle,
            Key,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "stateKeyHash" | "state_key_hash" => Ok(GeneratedField::StateKeyHash),
                            "handle" => Ok(GeneratedField::Handle),
                            "key" => Ok(GeneratedField::Key),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteTableItem;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.DeleteTableItem")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteTableItem, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut state_key_hash__ = None;
                let mut handle__ = None;
                let mut key__ = None;
                let mut data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::StateKeyHash => {
                            if state_key_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateKeyHash"));
                            }
                            state_key_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Handle => {
                            if handle__.is_some() {
                                return Err(serde::de::Error::duplicate_field("handle"));
                            }
                            handle__ = Some(map.next_value()?);
                        }
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map.next_value()?);
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map.next_value()?;
                        }
                    }
                }
                Ok(DeleteTableItem {
                    state_key_hash: state_key_hash__.unwrap_or_default(),
                    handle: handle__.unwrap_or_default(),
                    key: key__.unwrap_or_default(),
                    data: data__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.DeleteTableItem", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DirectWriteSet {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.write_set_change.is_empty() {
            len += 1;
        }
        if !self.events.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.DirectWriteSet", len)?;
        if !self.write_set_change.is_empty() {
            struct_ser.serialize_field("writeSetChange", &self.write_set_change)?;
        }
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DirectWriteSet {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "write_set_change",
            "writeSetChange",
            "events",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            WriteSetChange,
            Events,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "writeSetChange" | "write_set_change" => Ok(GeneratedField::WriteSetChange),
                            "events" => Ok(GeneratedField::Events),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DirectWriteSet;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.DirectWriteSet")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DirectWriteSet, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut write_set_change__ = None;
                let mut events__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::WriteSetChange => {
                            if write_set_change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeSetChange"));
                            }
                            write_set_change__ = Some(map.next_value()?);
                        }
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DirectWriteSet {
                    write_set_change: write_set_change__.unwrap_or_default(),
                    events: events__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.DirectWriteSet", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Ed25519 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signature.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.Ed25519", len)?;
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Ed25519 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Ed25519;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.Ed25519")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Ed25519, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Ed25519 {
                    signature: signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.Ed25519", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Ed25519Signature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.public_key.is_empty() {
            len += 1;
        }
        if !self.signature.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.Ed25519Signature", len)?;
        if !self.public_key.is_empty() {
            struct_ser.serialize_field("publicKey", pbjson::private::base64::encode(&self.public_key).as_str())?;
        }
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Ed25519Signature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "public_key",
            "publicKey",
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PublicKey,
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "publicKey" | "public_key" => Ok(GeneratedField::PublicKey),
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Ed25519Signature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.Ed25519Signature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Ed25519Signature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut public_key__ = None;
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::PublicKey => {
                            if public_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKey"));
                            }
                            public_key__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Ed25519Signature {
                    public_key: public_key__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.Ed25519Signature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EntryFunctionId {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.module.is_some() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.EntryFunctionId", len)?;
        if let Some(v) = self.module.as_ref() {
            struct_ser.serialize_field("module", v)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EntryFunctionId {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "module",
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Module,
            Name,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "module" => Ok(GeneratedField::Module),
                            "name" => Ok(GeneratedField::Name),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EntryFunctionId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.EntryFunctionId")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EntryFunctionId, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut module__ = None;
                let mut name__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Module => {
                            if module__.is_some() {
                                return Err(serde::de::Error::duplicate_field("module"));
                            }
                            module__ = map.next_value()?;
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(EntryFunctionId {
                    module: module__,
                    name: name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.EntryFunctionId", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EntryFunctionPayload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.function.is_some() {
            len += 1;
        }
        if !self.type_arguments.is_empty() {
            len += 1;
        }
        if !self.arguments.is_empty() {
            len += 1;
        }
        if !self.entry_function_id_str.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.EntryFunctionPayload", len)?;
        if let Some(v) = self.function.as_ref() {
            struct_ser.serialize_field("function", v)?;
        }
        if !self.type_arguments.is_empty() {
            struct_ser.serialize_field("typeArguments", &self.type_arguments)?;
        }
        if !self.arguments.is_empty() {
            struct_ser.serialize_field("arguments", &self.arguments)?;
        }
        if !self.entry_function_id_str.is_empty() {
            struct_ser.serialize_field("entryFunctionIdStr", &self.entry_function_id_str)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EntryFunctionPayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "function",
            "type_arguments",
            "typeArguments",
            "arguments",
            "entry_function_id_str",
            "entryFunctionIdStr",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Function,
            TypeArguments,
            Arguments,
            EntryFunctionIdStr,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "function" => Ok(GeneratedField::Function),
                            "typeArguments" | "type_arguments" => Ok(GeneratedField::TypeArguments),
                            "arguments" => Ok(GeneratedField::Arguments),
                            "entryFunctionIdStr" | "entry_function_id_str" => Ok(GeneratedField::EntryFunctionIdStr),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EntryFunctionPayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.EntryFunctionPayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EntryFunctionPayload, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut function__ = None;
                let mut type_arguments__ = None;
                let mut arguments__ = None;
                let mut entry_function_id_str__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Function => {
                            if function__.is_some() {
                                return Err(serde::de::Error::duplicate_field("function"));
                            }
                            function__ = map.next_value()?;
                        }
                        GeneratedField::TypeArguments => {
                            if type_arguments__.is_some() {
                                return Err(serde::de::Error::duplicate_field("typeArguments"));
                            }
                            type_arguments__ = Some(map.next_value()?);
                        }
                        GeneratedField::Arguments => {
                            if arguments__.is_some() {
                                return Err(serde::de::Error::duplicate_field("arguments"));
                            }
                            arguments__ = Some(map.next_value()?);
                        }
                        GeneratedField::EntryFunctionIdStr => {
                            if entry_function_id_str__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entryFunctionIdStr"));
                            }
                            entry_function_id_str__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(EntryFunctionPayload {
                    function: function__,
                    type_arguments: type_arguments__.unwrap_or_default(),
                    arguments: arguments__.unwrap_or_default(),
                    entry_function_id_str: entry_function_id_str__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.EntryFunctionPayload", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Event {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.key.is_some() {
            len += 1;
        }
        if self.sequence_number != 0 {
            len += 1;
        }
        if self.r#type.is_some() {
            len += 1;
        }
        if !self.type_str.is_empty() {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.Event", len)?;
        if let Some(v) = self.key.as_ref() {
            struct_ser.serialize_field("key", v)?;
        }
        if self.sequence_number != 0 {
            struct_ser.serialize_field("sequenceNumber", ToString::to_string(&self.sequence_number).as_str())?;
        }
        if let Some(v) = self.r#type.as_ref() {
            struct_ser.serialize_field("type", v)?;
        }
        if !self.type_str.is_empty() {
            struct_ser.serialize_field("typeStr", &self.type_str)?;
        }
        if !self.data.is_empty() {
            struct_ser.serialize_field("data", &self.data)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Event {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "sequence_number",
            "sequenceNumber",
            "type",
            "type_str",
            "typeStr",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            SequenceNumber,
            Type,
            TypeStr,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "sequenceNumber" | "sequence_number" => Ok(GeneratedField::SequenceNumber),
                            "type" => Ok(GeneratedField::Type),
                            "typeStr" | "type_str" => Ok(GeneratedField::TypeStr),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Event;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.Event")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Event, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut sequence_number__ = None;
                let mut r#type__ = None;
                let mut type_str__ = None;
                let mut data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = map.next_value()?;
                        }
                        GeneratedField::SequenceNumber => {
                            if sequence_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequenceNumber"));
                            }
                            sequence_number__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = map.next_value()?;
                        }
                        GeneratedField::TypeStr => {
                            if type_str__.is_some() {
                                return Err(serde::de::Error::duplicate_field("typeStr"));
                            }
                            type_str__ = Some(map.next_value()?);
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(Event {
                    key: key__,
                    sequence_number: sequence_number__.unwrap_or_default(),
                    r#type: r#type__,
                    type_str: type_str__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.Event", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EventKey {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.creation_number != 0 {
            len += 1;
        }
        if !self.account_address.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.EventKey", len)?;
        if self.creation_number != 0 {
            struct_ser.serialize_field("creationNumber", ToString::to_string(&self.creation_number).as_str())?;
        }
        if !self.account_address.is_empty() {
            struct_ser.serialize_field("accountAddress", &self.account_address)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EventKey {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "creation_number",
            "creationNumber",
            "account_address",
            "accountAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CreationNumber,
            AccountAddress,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "creationNumber" | "creation_number" => Ok(GeneratedField::CreationNumber),
                            "accountAddress" | "account_address" => Ok(GeneratedField::AccountAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EventKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.EventKey")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventKey, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut creation_number__ = None;
                let mut account_address__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::CreationNumber => {
                            if creation_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("creationNumber"));
                            }
                            creation_number__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::AccountAddress => {
                            if account_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("accountAddress"));
                            }
                            account_address__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(EventKey {
                    creation_number: creation_number__.unwrap_or_default(),
                    account_address: account_address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.EventKey", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EventSizeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.type_tag_bytes != 0 {
            len += 1;
        }
        if self.total_bytes != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.EventSizeInfo", len)?;
        if self.type_tag_bytes != 0 {
            struct_ser.serialize_field("typeTagBytes", &self.type_tag_bytes)?;
        }
        if self.total_bytes != 0 {
            struct_ser.serialize_field("totalBytes", &self.total_bytes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EventSizeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type_tag_bytes",
            "typeTagBytes",
            "total_bytes",
            "totalBytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TypeTagBytes,
            TotalBytes,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "typeTagBytes" | "type_tag_bytes" => Ok(GeneratedField::TypeTagBytes),
                            "totalBytes" | "total_bytes" => Ok(GeneratedField::TotalBytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EventSizeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.EventSizeInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventSizeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut type_tag_bytes__ = None;
                let mut total_bytes__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TypeTagBytes => {
                            if type_tag_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("typeTagBytes"));
                            }
                            type_tag_bytes__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TotalBytes => {
                            if total_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalBytes"));
                            }
                            total_bytes__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(EventSizeInfo {
                    type_tag_bytes: type_tag_bytes__.unwrap_or_default(),
                    total_bytes: total_bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.EventSizeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExtraConfigV1 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.multisig_address.is_some() {
            len += 1;
        }
        if self.replay_protection_nonce.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ExtraConfigV1", len)?;
        if let Some(v) = self.multisig_address.as_ref() {
            struct_ser.serialize_field("multisigAddress", v)?;
        }
        if let Some(v) = self.replay_protection_nonce.as_ref() {
            struct_ser.serialize_field("replayProtectionNonce", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExtraConfigV1 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "multisig_address",
            "multisigAddress",
            "replay_protection_nonce",
            "replayProtectionNonce",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MultisigAddress,
            ReplayProtectionNonce,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "multisigAddress" | "multisig_address" => Ok(GeneratedField::MultisigAddress),
                            "replayProtectionNonce" | "replay_protection_nonce" => Ok(GeneratedField::ReplayProtectionNonce),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExtraConfigV1;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ExtraConfigV1")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ExtraConfigV1, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut multisig_address__ = None;
                let mut replay_protection_nonce__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::MultisigAddress => {
                            if multisig_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multisigAddress"));
                            }
                            multisig_address__ = map.next_value()?;
                        }
                        GeneratedField::ReplayProtectionNonce => {
                            if replay_protection_nonce__.is_some() {
                                return Err(serde::de::Error::duplicate_field("replayProtectionNonce"));
                            }
                            replay_protection_nonce__ =
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(ExtraConfigV1 {
                    multisig_address: multisig_address__,
                    replay_protection_nonce: replay_protection_nonce__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ExtraConfigV1", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FeePayerSignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.sender.is_some() {
            len += 1;
        }
        if !self.secondary_signer_addresses.is_empty() {
            len += 1;
        }
        if !self.secondary_signers.is_empty() {
            len += 1;
        }
        if !self.fee_payer_address.is_empty() {
            len += 1;
        }
        if self.fee_payer_signer.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.FeePayerSignature", len)?;
        if let Some(v) = self.sender.as_ref() {
            struct_ser.serialize_field("sender", v)?;
        }
        if !self.secondary_signer_addresses.is_empty() {
            struct_ser.serialize_field("secondarySignerAddresses", &self.secondary_signer_addresses)?;
        }
        if !self.secondary_signers.is_empty() {
            struct_ser.serialize_field("secondarySigners", &self.secondary_signers)?;
        }
        if !self.fee_payer_address.is_empty() {
            struct_ser.serialize_field("feePayerAddress", &self.fee_payer_address)?;
        }
        if let Some(v) = self.fee_payer_signer.as_ref() {
            struct_ser.serialize_field("feePayerSigner", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FeePayerSignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sender",
            "secondary_signer_addresses",
            "secondarySignerAddresses",
            "secondary_signers",
            "secondarySigners",
            "fee_payer_address",
            "feePayerAddress",
            "fee_payer_signer",
            "feePayerSigner",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Sender,
            SecondarySignerAddresses,
            SecondarySigners,
            FeePayerAddress,
            FeePayerSigner,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "sender" => Ok(GeneratedField::Sender),
                            "secondarySignerAddresses" | "secondary_signer_addresses" => Ok(GeneratedField::SecondarySignerAddresses),
                            "secondarySigners" | "secondary_signers" => Ok(GeneratedField::SecondarySigners),
                            "feePayerAddress" | "fee_payer_address" => Ok(GeneratedField::FeePayerAddress),
                            "feePayerSigner" | "fee_payer_signer" => Ok(GeneratedField::FeePayerSigner),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FeePayerSignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.FeePayerSignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<FeePayerSignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sender__ = None;
                let mut secondary_signer_addresses__ = None;
                let mut secondary_signers__ = None;
                let mut fee_payer_address__ = None;
                let mut fee_payer_signer__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Sender => {
                            if sender__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sender"));
                            }
                            sender__ = map.next_value()?;
                        }
                        GeneratedField::SecondarySignerAddresses => {
                            if secondary_signer_addresses__.is_some() {
                                return Err(serde::de::Error::duplicate_field("secondarySignerAddresses"));
                            }
                            secondary_signer_addresses__ = Some(map.next_value()?);
                        }
                        GeneratedField::SecondarySigners => {
                            if secondary_signers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("secondarySigners"));
                            }
                            secondary_signers__ = Some(map.next_value()?);
                        }
                        GeneratedField::FeePayerAddress => {
                            if fee_payer_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feePayerAddress"));
                            }
                            fee_payer_address__ = Some(map.next_value()?);
                        }
                        GeneratedField::FeePayerSigner => {
                            if fee_payer_signer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feePayerSigner"));
                            }
                            fee_payer_signer__ = map.next_value()?;
                        }
                    }
                }
                Ok(FeePayerSignature {
                    sender: sender__,
                    secondary_signer_addresses: secondary_signer_addresses__.unwrap_or_default(),
                    secondary_signers: secondary_signers__.unwrap_or_default(),
                    fee_payer_address: fee_payer_address__.unwrap_or_default(),
                    fee_payer_signer: fee_payer_signer__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.FeePayerSignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GenesisTransaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.payload.is_some() {
            len += 1;
        }
        if !self.events.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.GenesisTransaction", len)?;
        if let Some(v) = self.payload.as_ref() {
            struct_ser.serialize_field("payload", v)?;
        }
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenesisTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "payload",
            "events",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Payload,
            Events,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "payload" => Ok(GeneratedField::Payload),
                            "events" => Ok(GeneratedField::Events),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenesisTransaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.GenesisTransaction")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GenesisTransaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut payload__ = None;
                let mut events__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = map.next_value()?;
                        }
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(GenesisTransaction {
                    payload: payload__,
                    events: events__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.GenesisTransaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IndexedSignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.index != 0 {
            len += 1;
        }
        if self.signature.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.IndexedSignature", len)?;
        if self.index != 0 {
            struct_ser.serialize_field("index", &self.index)?;
        }
        if let Some(v) = self.signature.as_ref() {
            struct_ser.serialize_field("signature", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IndexedSignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "index",
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Index,
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "index" => Ok(GeneratedField::Index),
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IndexedSignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.IndexedSignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<IndexedSignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut index__ = None;
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = map.next_value()?;
                        }
                    }
                }
                Ok(IndexedSignature {
                    index: index__.unwrap_or_default(),
                    signature: signature__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.IndexedSignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Keyless {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signature.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.Keyless", len)?;
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Keyless {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Keyless;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.Keyless")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Keyless, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Keyless {
                    signature: signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.Keyless", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveAbility {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "MOVE_ABILITY_UNSPECIFIED",
            Self::Copy => "MOVE_ABILITY_COPY",
            Self::Drop => "MOVE_ABILITY_DROP",
            Self::Store => "MOVE_ABILITY_STORE",
            Self::Key => "MOVE_ABILITY_KEY",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for MoveAbility {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MOVE_ABILITY_UNSPECIFIED",
            "MOVE_ABILITY_COPY",
            "MOVE_ABILITY_DROP",
            "MOVE_ABILITY_STORE",
            "MOVE_ABILITY_KEY",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveAbility;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(MoveAbility::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(MoveAbility::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "MOVE_ABILITY_UNSPECIFIED" => Ok(MoveAbility::Unspecified),
                    "MOVE_ABILITY_COPY" => Ok(MoveAbility::Copy),
                    "MOVE_ABILITY_DROP" => Ok(MoveAbility::Drop),
                    "MOVE_ABILITY_STORE" => Ok(MoveAbility::Store),
                    "MOVE_ABILITY_KEY" => Ok(MoveAbility::Key),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for MoveFunction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if self.visibility != 0 {
            len += 1;
        }
        if self.is_entry {
            len += 1;
        }
        if !self.generic_type_params.is_empty() {
            len += 1;
        }
        if !self.params.is_empty() {
            len += 1;
        }
        if !self.r#return.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveFunction", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if self.visibility != 0 {
            let v = move_function::Visibility::from_i32(self.visibility)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.visibility)))?;
            struct_ser.serialize_field("visibility", &v)?;
        }
        if self.is_entry {
            struct_ser.serialize_field("isEntry", &self.is_entry)?;
        }
        if !self.generic_type_params.is_empty() {
            struct_ser.serialize_field("genericTypeParams", &self.generic_type_params)?;
        }
        if !self.params.is_empty() {
            struct_ser.serialize_field("params", &self.params)?;
        }
        if !self.r#return.is_empty() {
            struct_ser.serialize_field("return", &self.r#return)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveFunction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "visibility",
            "is_entry",
            "isEntry",
            "generic_type_params",
            "genericTypeParams",
            "params",
            "return",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Visibility,
            IsEntry,
            GenericTypeParams,
            Params,
            Return,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "visibility" => Ok(GeneratedField::Visibility),
                            "isEntry" | "is_entry" => Ok(GeneratedField::IsEntry),
                            "genericTypeParams" | "generic_type_params" => Ok(GeneratedField::GenericTypeParams),
                            "params" => Ok(GeneratedField::Params),
                            "return" => Ok(GeneratedField::Return),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveFunction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveFunction")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveFunction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut visibility__ = None;
                let mut is_entry__ = None;
                let mut generic_type_params__ = None;
                let mut params__ = None;
                let mut r#return__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Visibility => {
                            if visibility__.is_some() {
                                return Err(serde::de::Error::duplicate_field("visibility"));
                            }
                            visibility__ = Some(map.next_value::<move_function::Visibility>()? as i32);
                        }
                        GeneratedField::IsEntry => {
                            if is_entry__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isEntry"));
                            }
                            is_entry__ = Some(map.next_value()?);
                        }
                        GeneratedField::GenericTypeParams => {
                            if generic_type_params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genericTypeParams"));
                            }
                            generic_type_params__ = Some(map.next_value()?);
                        }
                        GeneratedField::Params => {
                            if params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params__ = Some(map.next_value()?);
                        }
                        GeneratedField::Return => {
                            if r#return__.is_some() {
                                return Err(serde::de::Error::duplicate_field("return"));
                            }
                            r#return__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveFunction {
                    name: name__.unwrap_or_default(),
                    visibility: visibility__.unwrap_or_default(),
                    is_entry: is_entry__.unwrap_or_default(),
                    generic_type_params: generic_type_params__.unwrap_or_default(),
                    params: params__.unwrap_or_default(),
                    r#return: r#return__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveFunction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for move_function::Visibility {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "VISIBILITY_UNSPECIFIED",
            Self::Private => "VISIBILITY_PRIVATE",
            Self::Public => "VISIBILITY_PUBLIC",
            Self::Friend => "VISIBILITY_FRIEND",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for move_function::Visibility {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "VISIBILITY_UNSPECIFIED",
            "VISIBILITY_PRIVATE",
            "VISIBILITY_PUBLIC",
            "VISIBILITY_FRIEND",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = move_function::Visibility;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(move_function::Visibility::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(move_function::Visibility::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "VISIBILITY_UNSPECIFIED" => Ok(move_function::Visibility::Unspecified),
                    "VISIBILITY_PRIVATE" => Ok(move_function::Visibility::Private),
                    "VISIBILITY_PUBLIC" => Ok(move_function::Visibility::Public),
                    "VISIBILITY_FRIEND" => Ok(move_function::Visibility::Friend),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for MoveFunctionGenericTypeParam {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.constraints.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveFunctionGenericTypeParam", len)?;
        if !self.constraints.is_empty() {
            let v = self.constraints.iter().cloned().map(|v| {
                MoveAbility::from_i32(v)
                    .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                }).collect::<Result<Vec<_>, _>>()?;
            struct_ser.serialize_field("constraints", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveFunctionGenericTypeParam {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "constraints",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Constraints,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "constraints" => Ok(GeneratedField::Constraints),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveFunctionGenericTypeParam;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveFunctionGenericTypeParam")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveFunctionGenericTypeParam, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut constraints__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Constraints => {
                            if constraints__.is_some() {
                                return Err(serde::de::Error::duplicate_field("constraints"));
                            }
                            constraints__ = Some(map.next_value::<Vec<MoveAbility>>()?.into_iter().map(|x| x as i32).collect());
                        }
                    }
                }
                Ok(MoveFunctionGenericTypeParam {
                    constraints: constraints__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveFunctionGenericTypeParam", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveModule {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.address.is_empty() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.friends.is_empty() {
            len += 1;
        }
        if !self.exposed_functions.is_empty() {
            len += 1;
        }
        if !self.structs.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveModule", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.friends.is_empty() {
            struct_ser.serialize_field("friends", &self.friends)?;
        }
        if !self.exposed_functions.is_empty() {
            struct_ser.serialize_field("exposedFunctions", &self.exposed_functions)?;
        }
        if !self.structs.is_empty() {
            struct_ser.serialize_field("structs", &self.structs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveModule {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "name",
            "friends",
            "exposed_functions",
            "exposedFunctions",
            "structs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            Name,
            Friends,
            ExposedFunctions,
            Structs,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "name" => Ok(GeneratedField::Name),
                            "friends" => Ok(GeneratedField::Friends),
                            "exposedFunctions" | "exposed_functions" => Ok(GeneratedField::ExposedFunctions),
                            "structs" => Ok(GeneratedField::Structs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveModule;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveModule")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveModule, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut name__ = None;
                let mut friends__ = None;
                let mut exposed_functions__ = None;
                let mut structs__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Friends => {
                            if friends__.is_some() {
                                return Err(serde::de::Error::duplicate_field("friends"));
                            }
                            friends__ = Some(map.next_value()?);
                        }
                        GeneratedField::ExposedFunctions => {
                            if exposed_functions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("exposedFunctions"));
                            }
                            exposed_functions__ = Some(map.next_value()?);
                        }
                        GeneratedField::Structs => {
                            if structs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("structs"));
                            }
                            structs__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveModule {
                    address: address__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    friends: friends__.unwrap_or_default(),
                    exposed_functions: exposed_functions__.unwrap_or_default(),
                    structs: structs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveModule", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveModuleBytecode {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.bytecode.is_empty() {
            len += 1;
        }
        if self.abi.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveModuleBytecode", len)?;
        if !self.bytecode.is_empty() {
            struct_ser.serialize_field("bytecode", pbjson::private::base64::encode(&self.bytecode).as_str())?;
        }
        if let Some(v) = self.abi.as_ref() {
            struct_ser.serialize_field("abi", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveModuleBytecode {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bytecode",
            "abi",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bytecode,
            Abi,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "bytecode" => Ok(GeneratedField::Bytecode),
                            "abi" => Ok(GeneratedField::Abi),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveModuleBytecode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveModuleBytecode")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveModuleBytecode, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bytecode__ = None;
                let mut abi__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Bytecode => {
                            if bytecode__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bytecode"));
                            }
                            bytecode__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Abi => {
                            if abi__.is_some() {
                                return Err(serde::de::Error::duplicate_field("abi"));
                            }
                            abi__ = map.next_value()?;
                        }
                    }
                }
                Ok(MoveModuleBytecode {
                    bytecode: bytecode__.unwrap_or_default(),
                    abi: abi__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveModuleBytecode", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveModuleId {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.address.is_empty() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveModuleId", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveModuleId {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            Name,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "name" => Ok(GeneratedField::Name),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveModuleId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveModuleId")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveModuleId, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut name__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveModuleId {
                    address: address__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveModuleId", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveScriptBytecode {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.bytecode.is_empty() {
            len += 1;
        }
        if self.abi.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveScriptBytecode", len)?;
        if !self.bytecode.is_empty() {
            struct_ser.serialize_field("bytecode", pbjson::private::base64::encode(&self.bytecode).as_str())?;
        }
        if let Some(v) = self.abi.as_ref() {
            struct_ser.serialize_field("abi", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveScriptBytecode {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bytecode",
            "abi",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bytecode,
            Abi,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "bytecode" => Ok(GeneratedField::Bytecode),
                            "abi" => Ok(GeneratedField::Abi),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveScriptBytecode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveScriptBytecode")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveScriptBytecode, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bytecode__ = None;
                let mut abi__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Bytecode => {
                            if bytecode__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bytecode"));
                            }
                            bytecode__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Abi => {
                            if abi__.is_some() {
                                return Err(serde::de::Error::duplicate_field("abi"));
                            }
                            abi__ = map.next_value()?;
                        }
                    }
                }
                Ok(MoveScriptBytecode {
                    bytecode: bytecode__.unwrap_or_default(),
                    abi: abi__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveScriptBytecode", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveStruct {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if self.is_native {
            len += 1;
        }
        if self.is_event {
            len += 1;
        }
        if !self.abilities.is_empty() {
            len += 1;
        }
        if !self.generic_type_params.is_empty() {
            len += 1;
        }
        if !self.fields.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveStruct", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if self.is_native {
            struct_ser.serialize_field("isNative", &self.is_native)?;
        }
        if self.is_event {
            struct_ser.serialize_field("isEvent", &self.is_event)?;
        }
        if !self.abilities.is_empty() {
            let v = self.abilities.iter().cloned().map(|v| {
                MoveAbility::from_i32(v)
                    .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                }).collect::<Result<Vec<_>, _>>()?;
            struct_ser.serialize_field("abilities", &v)?;
        }
        if !self.generic_type_params.is_empty() {
            struct_ser.serialize_field("genericTypeParams", &self.generic_type_params)?;
        }
        if !self.fields.is_empty() {
            struct_ser.serialize_field("fields", &self.fields)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveStruct {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "is_native",
            "isNative",
            "is_event",
            "isEvent",
            "abilities",
            "generic_type_params",
            "genericTypeParams",
            "fields",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            IsNative,
            IsEvent,
            Abilities,
            GenericTypeParams,
            Fields,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "isNative" | "is_native" => Ok(GeneratedField::IsNative),
                            "isEvent" | "is_event" => Ok(GeneratedField::IsEvent),
                            "abilities" => Ok(GeneratedField::Abilities),
                            "genericTypeParams" | "generic_type_params" => Ok(GeneratedField::GenericTypeParams),
                            "fields" => Ok(GeneratedField::Fields),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveStruct;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveStruct")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveStruct, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut is_native__ = None;
                let mut is_event__ = None;
                let mut abilities__ = None;
                let mut generic_type_params__ = None;
                let mut fields__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::IsNative => {
                            if is_native__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isNative"));
                            }
                            is_native__ = Some(map.next_value()?);
                        }
                        GeneratedField::IsEvent => {
                            if is_event__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isEvent"));
                            }
                            is_event__ = Some(map.next_value()?);
                        }
                        GeneratedField::Abilities => {
                            if abilities__.is_some() {
                                return Err(serde::de::Error::duplicate_field("abilities"));
                            }
                            abilities__ = Some(map.next_value::<Vec<MoveAbility>>()?.into_iter().map(|x| x as i32).collect());
                        }
                        GeneratedField::GenericTypeParams => {
                            if generic_type_params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genericTypeParams"));
                            }
                            generic_type_params__ = Some(map.next_value()?);
                        }
                        GeneratedField::Fields => {
                            if fields__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fields"));
                            }
                            fields__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveStruct {
                    name: name__.unwrap_or_default(),
                    is_native: is_native__.unwrap_or_default(),
                    is_event: is_event__.unwrap_or_default(),
                    abilities: abilities__.unwrap_or_default(),
                    generic_type_params: generic_type_params__.unwrap_or_default(),
                    fields: fields__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveStruct", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveStructField {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if self.r#type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveStructField", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if let Some(v) = self.r#type.as_ref() {
            struct_ser.serialize_field("type", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveStructField {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "type",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Type,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "type" => Ok(GeneratedField::Type),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveStructField;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveStructField")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveStructField, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut r#type__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = map.next_value()?;
                        }
                    }
                }
                Ok(MoveStructField {
                    name: name__.unwrap_or_default(),
                    r#type: r#type__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveStructField", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveStructGenericTypeParam {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.constraints.is_empty() {
            len += 1;
        }
        if self.is_phantom {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveStructGenericTypeParam", len)?;
        if !self.constraints.is_empty() {
            let v = self.constraints.iter().cloned().map(|v| {
                MoveAbility::from_i32(v)
                    .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                }).collect::<Result<Vec<_>, _>>()?;
            struct_ser.serialize_field("constraints", &v)?;
        }
        if self.is_phantom {
            struct_ser.serialize_field("isPhantom", &self.is_phantom)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveStructGenericTypeParam {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "constraints",
            "is_phantom",
            "isPhantom",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Constraints,
            IsPhantom,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "constraints" => Ok(GeneratedField::Constraints),
                            "isPhantom" | "is_phantom" => Ok(GeneratedField::IsPhantom),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveStructGenericTypeParam;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveStructGenericTypeParam")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveStructGenericTypeParam, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut constraints__ = None;
                let mut is_phantom__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Constraints => {
                            if constraints__.is_some() {
                                return Err(serde::de::Error::duplicate_field("constraints"));
                            }
                            constraints__ = Some(map.next_value::<Vec<MoveAbility>>()?.into_iter().map(|x| x as i32).collect());
                        }
                        GeneratedField::IsPhantom => {
                            if is_phantom__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isPhantom"));
                            }
                            is_phantom__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveStructGenericTypeParam {
                    constraints: constraints__.unwrap_or_default(),
                    is_phantom: is_phantom__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveStructGenericTypeParam", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveStructTag {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.address.is_empty() {
            len += 1;
        }
        if !self.module.is_empty() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.generic_type_params.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveStructTag", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.module.is_empty() {
            struct_ser.serialize_field("module", &self.module)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.generic_type_params.is_empty() {
            struct_ser.serialize_field("genericTypeParams", &self.generic_type_params)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveStructTag {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "module",
            "name",
            "generic_type_params",
            "genericTypeParams",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            Module,
            Name,
            GenericTypeParams,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "module" => Ok(GeneratedField::Module),
                            "name" => Ok(GeneratedField::Name),
                            "genericTypeParams" | "generic_type_params" => Ok(GeneratedField::GenericTypeParams),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveStructTag;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveStructTag")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveStructTag, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut module__ = None;
                let mut name__ = None;
                let mut generic_type_params__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        }
                        GeneratedField::Module => {
                            if module__.is_some() {
                                return Err(serde::de::Error::duplicate_field("module"));
                            }
                            module__ = Some(map.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::GenericTypeParams => {
                            if generic_type_params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genericTypeParams"));
                            }
                            generic_type_params__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveStructTag {
                    address: address__.unwrap_or_default(),
                    module: module__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    generic_type_params: generic_type_params__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveStructTag", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if self.content.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveType", len)?;
        if self.r#type != 0 {
            let v = MoveTypes::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if let Some(v) = self.content.as_ref() {
            match v {
                move_type::Content::Vector(v) => {
                    struct_ser.serialize_field("vector", v)?;
                }
                move_type::Content::Struct(v) => {
                    struct_ser.serialize_field("struct", v)?;
                }
                move_type::Content::GenericTypeParamIndex(v) => {
                    struct_ser.serialize_field("genericTypeParamIndex", v)?;
                }
                move_type::Content::Reference(v) => {
                    struct_ser.serialize_field("reference", v)?;
                }
                move_type::Content::Unparsable(v) => {
                    struct_ser.serialize_field("unparsable", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveType {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "vector",
            "struct",
            "generic_type_param_index",
            "genericTypeParamIndex",
            "reference",
            "unparsable",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Vector,
            Struct,
            GenericTypeParamIndex,
            Reference,
            Unparsable,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "vector" => Ok(GeneratedField::Vector),
                            "struct" => Ok(GeneratedField::Struct),
                            "genericTypeParamIndex" | "generic_type_param_index" => Ok(GeneratedField::GenericTypeParamIndex),
                            "reference" => Ok(GeneratedField::Reference),
                            "unparsable" => Ok(GeneratedField::Unparsable),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveType")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveType, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut content__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<MoveTypes>()? as i32);
                        }
                        GeneratedField::Vector => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("vector"));
                            }
                            content__ = map.next_value::<::std::option::Option<_>>()?.map(move_type::Content::Vector)
;
                        }
                        GeneratedField::Struct => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("struct"));
                            }
                            content__ = map.next_value::<::std::option::Option<_>>()?.map(move_type::Content::Struct)
;
                        }
                        GeneratedField::GenericTypeParamIndex => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genericTypeParamIndex"));
                            }
                            content__ = map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| move_type::Content::GenericTypeParamIndex(x.0));
                        }
                        GeneratedField::Reference => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reference"));
                            }
                            content__ = map.next_value::<::std::option::Option<_>>()?.map(move_type::Content::Reference)
;
                        }
                        GeneratedField::Unparsable => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("unparsable"));
                            }
                            content__ = map.next_value::<::std::option::Option<_>>()?.map(move_type::Content::Unparsable);
                        }
                    }
                }
                Ok(MoveType {
                    r#type: r#type__.unwrap_or_default(),
                    content: content__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveType", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for move_type::ReferenceType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.mutable {
            len += 1;
        }
        if self.to.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MoveType.ReferenceType", len)?;
        if self.mutable {
            struct_ser.serialize_field("mutable", &self.mutable)?;
        }
        if let Some(v) = self.to.as_ref() {
            struct_ser.serialize_field("to", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for move_type::ReferenceType {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "mutable",
            "to",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Mutable,
            To,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "mutable" => Ok(GeneratedField::Mutable),
                            "to" => Ok(GeneratedField::To),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = move_type::ReferenceType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MoveType.ReferenceType")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<move_type::ReferenceType, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut mutable__ = None;
                let mut to__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Mutable => {
                            if mutable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mutable"));
                            }
                            mutable__ = Some(map.next_value()?);
                        }
                        GeneratedField::To => {
                            if to__.is_some() {
                                return Err(serde::de::Error::duplicate_field("to"));
                            }
                            to__ = map.next_value()?;
                        }
                    }
                }
                Ok(move_type::ReferenceType {
                    mutable: mutable__.unwrap_or_default(),
                    to: to__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MoveType.ReferenceType", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveTypes {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "MOVE_TYPES_UNSPECIFIED",
            Self::Bool => "MOVE_TYPES_BOOL",
            Self::U8 => "MOVE_TYPES_U8",
            Self::U16 => "MOVE_TYPES_U16",
            Self::U32 => "MOVE_TYPES_U32",
            Self::U64 => "MOVE_TYPES_U64",
            Self::U128 => "MOVE_TYPES_U128",
            Self::U256 => "MOVE_TYPES_U256",
            Self::Address => "MOVE_TYPES_ADDRESS",
            Self::Signer => "MOVE_TYPES_SIGNER",
            Self::Vector => "MOVE_TYPES_VECTOR",
            Self::Struct => "MOVE_TYPES_STRUCT",
            Self::GenericTypeParam => "MOVE_TYPES_GENERIC_TYPE_PARAM",
            Self::Reference => "MOVE_TYPES_REFERENCE",
            Self::Unparsable => "MOVE_TYPES_UNPARSABLE",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for MoveTypes {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MOVE_TYPES_UNSPECIFIED",
            "MOVE_TYPES_BOOL",
            "MOVE_TYPES_U8",
            "MOVE_TYPES_U16",
            "MOVE_TYPES_U32",
            "MOVE_TYPES_U64",
            "MOVE_TYPES_U128",
            "MOVE_TYPES_U256",
            "MOVE_TYPES_ADDRESS",
            "MOVE_TYPES_SIGNER",
            "MOVE_TYPES_VECTOR",
            "MOVE_TYPES_STRUCT",
            "MOVE_TYPES_GENERIC_TYPE_PARAM",
            "MOVE_TYPES_REFERENCE",
            "MOVE_TYPES_UNPARSABLE",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveTypes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(MoveTypes::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(MoveTypes::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "MOVE_TYPES_UNSPECIFIED" => Ok(MoveTypes::Unspecified),
                    "MOVE_TYPES_BOOL" => Ok(MoveTypes::Bool),
                    "MOVE_TYPES_U8" => Ok(MoveTypes::U8),
                    "MOVE_TYPES_U16" => Ok(MoveTypes::U16),
                    "MOVE_TYPES_U32" => Ok(MoveTypes::U32),
                    "MOVE_TYPES_U64" => Ok(MoveTypes::U64),
                    "MOVE_TYPES_U128" => Ok(MoveTypes::U128),
                    "MOVE_TYPES_U256" => Ok(MoveTypes::U256),
                    "MOVE_TYPES_ADDRESS" => Ok(MoveTypes::Address),
                    "MOVE_TYPES_SIGNER" => Ok(MoveTypes::Signer),
                    "MOVE_TYPES_VECTOR" => Ok(MoveTypes::Vector),
                    "MOVE_TYPES_STRUCT" => Ok(MoveTypes::Struct),
                    "MOVE_TYPES_GENERIC_TYPE_PARAM" => Ok(MoveTypes::GenericTypeParam),
                    "MOVE_TYPES_REFERENCE" => Ok(MoveTypes::Reference),
                    "MOVE_TYPES_UNPARSABLE" => Ok(MoveTypes::Unparsable),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for MultiAgentSignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.sender.is_some() {
            len += 1;
        }
        if !self.secondary_signer_addresses.is_empty() {
            len += 1;
        }
        if !self.secondary_signers.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MultiAgentSignature", len)?;
        if let Some(v) = self.sender.as_ref() {
            struct_ser.serialize_field("sender", v)?;
        }
        if !self.secondary_signer_addresses.is_empty() {
            struct_ser.serialize_field("secondarySignerAddresses", &self.secondary_signer_addresses)?;
        }
        if !self.secondary_signers.is_empty() {
            struct_ser.serialize_field("secondarySigners", &self.secondary_signers)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MultiAgentSignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sender",
            "secondary_signer_addresses",
            "secondarySignerAddresses",
            "secondary_signers",
            "secondarySigners",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Sender,
            SecondarySignerAddresses,
            SecondarySigners,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "sender" => Ok(GeneratedField::Sender),
                            "secondarySignerAddresses" | "secondary_signer_addresses" => Ok(GeneratedField::SecondarySignerAddresses),
                            "secondarySigners" | "secondary_signers" => Ok(GeneratedField::SecondarySigners),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MultiAgentSignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MultiAgentSignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MultiAgentSignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sender__ = None;
                let mut secondary_signer_addresses__ = None;
                let mut secondary_signers__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Sender => {
                            if sender__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sender"));
                            }
                            sender__ = map.next_value()?;
                        }
                        GeneratedField::SecondarySignerAddresses => {
                            if secondary_signer_addresses__.is_some() {
                                return Err(serde::de::Error::duplicate_field("secondarySignerAddresses"));
                            }
                            secondary_signer_addresses__ = Some(map.next_value()?);
                        }
                        GeneratedField::SecondarySigners => {
                            if secondary_signers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("secondarySigners"));
                            }
                            secondary_signers__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MultiAgentSignature {
                    sender: sender__,
                    secondary_signer_addresses: secondary_signer_addresses__.unwrap_or_default(),
                    secondary_signers: secondary_signers__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MultiAgentSignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MultiEd25519Signature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.public_keys.is_empty() {
            len += 1;
        }
        if !self.signatures.is_empty() {
            len += 1;
        }
        if self.threshold != 0 {
            len += 1;
        }
        if !self.public_key_indices.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MultiEd25519Signature", len)?;
        if !self.public_keys.is_empty() {
            struct_ser.serialize_field("publicKeys", &self.public_keys.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if !self.signatures.is_empty() {
            struct_ser.serialize_field("signatures", &self.signatures.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if self.threshold != 0 {
            struct_ser.serialize_field("threshold", &self.threshold)?;
        }
        if !self.public_key_indices.is_empty() {
            struct_ser.serialize_field("publicKeyIndices", &self.public_key_indices)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MultiEd25519Signature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "public_keys",
            "publicKeys",
            "signatures",
            "threshold",
            "public_key_indices",
            "publicKeyIndices",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PublicKeys,
            Signatures,
            Threshold,
            PublicKeyIndices,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "publicKeys" | "public_keys" => Ok(GeneratedField::PublicKeys),
                            "signatures" => Ok(GeneratedField::Signatures),
                            "threshold" => Ok(GeneratedField::Threshold),
                            "publicKeyIndices" | "public_key_indices" => Ok(GeneratedField::PublicKeyIndices),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MultiEd25519Signature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MultiEd25519Signature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MultiEd25519Signature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut public_keys__ = None;
                let mut signatures__ = None;
                let mut threshold__ = None;
                let mut public_key_indices__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::PublicKeys => {
                            if public_keys__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKeys"));
                            }
                            public_keys__ =
                                Some(map.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ =
                                Some(map.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::Threshold => {
                            if threshold__.is_some() {
                                return Err(serde::de::Error::duplicate_field("threshold"));
                            }
                            threshold__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PublicKeyIndices => {
                            if public_key_indices__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKeyIndices"));
                            }
                            public_key_indices__ =
                                Some(map.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(MultiEd25519Signature {
                    public_keys: public_keys__.unwrap_or_default(),
                    signatures: signatures__.unwrap_or_default(),
                    threshold: threshold__.unwrap_or_default(),
                    public_key_indices: public_key_indices__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MultiEd25519Signature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MultiKeySignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.public_keys.is_empty() {
            len += 1;
        }
        if !self.signatures.is_empty() {
            len += 1;
        }
        if self.signatures_required != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MultiKeySignature", len)?;
        if !self.public_keys.is_empty() {
            struct_ser.serialize_field("publicKeys", &self.public_keys)?;
        }
        if !self.signatures.is_empty() {
            struct_ser.serialize_field("signatures", &self.signatures)?;
        }
        if self.signatures_required != 0 {
            struct_ser.serialize_field("signaturesRequired", &self.signatures_required)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MultiKeySignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "public_keys",
            "publicKeys",
            "signatures",
            "signatures_required",
            "signaturesRequired",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PublicKeys,
            Signatures,
            SignaturesRequired,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "publicKeys" | "public_keys" => Ok(GeneratedField::PublicKeys),
                            "signatures" => Ok(GeneratedField::Signatures),
                            "signaturesRequired" | "signatures_required" => Ok(GeneratedField::SignaturesRequired),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MultiKeySignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MultiKeySignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MultiKeySignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut public_keys__ = None;
                let mut signatures__ = None;
                let mut signatures_required__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::PublicKeys => {
                            if public_keys__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKeys"));
                            }
                            public_keys__ = Some(map.next_value()?);
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = Some(map.next_value()?);
                        }
                        GeneratedField::SignaturesRequired => {
                            if signatures_required__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signaturesRequired"));
                            }
                            signatures_required__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(MultiKeySignature {
                    public_keys: public_keys__.unwrap_or_default(),
                    signatures: signatures__.unwrap_or_default(),
                    signatures_required: signatures_required__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MultiKeySignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MultisigPayload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.multisig_address.is_empty() {
            len += 1;
        }
        if self.transaction_payload.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MultisigPayload", len)?;
        if !self.multisig_address.is_empty() {
            struct_ser.serialize_field("multisigAddress", &self.multisig_address)?;
        }
        if let Some(v) = self.transaction_payload.as_ref() {
            struct_ser.serialize_field("transactionPayload", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MultisigPayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "multisig_address",
            "multisigAddress",
            "transaction_payload",
            "transactionPayload",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MultisigAddress,
            TransactionPayload,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "multisigAddress" | "multisig_address" => Ok(GeneratedField::MultisigAddress),
                            "transactionPayload" | "transaction_payload" => Ok(GeneratedField::TransactionPayload),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MultisigPayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MultisigPayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MultisigPayload, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut multisig_address__ = None;
                let mut transaction_payload__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::MultisigAddress => {
                            if multisig_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multisigAddress"));
                            }
                            multisig_address__ = Some(map.next_value()?);
                        }
                        GeneratedField::TransactionPayload => {
                            if transaction_payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionPayload"));
                            }
                            transaction_payload__ = map.next_value()?;
                        }
                    }
                }
                Ok(MultisigPayload {
                    multisig_address: multisig_address__.unwrap_or_default(),
                    transaction_payload: transaction_payload__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MultisigPayload", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MultisigTransactionPayload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if self.payload.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.MultisigTransactionPayload", len)?;
        if self.r#type != 0 {
            let v = multisig_transaction_payload::Type::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if let Some(v) = self.payload.as_ref() {
            match v {
                multisig_transaction_payload::Payload::EntryFunctionPayload(v) => {
                    struct_ser.serialize_field("entryFunctionPayload", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MultisigTransactionPayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "entry_function_payload",
            "entryFunctionPayload",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            EntryFunctionPayload,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "entryFunctionPayload" | "entry_function_payload" => Ok(GeneratedField::EntryFunctionPayload),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MultisigTransactionPayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.MultisigTransactionPayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MultisigTransactionPayload, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut payload__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<multisig_transaction_payload::Type>()? as i32);
                        }
                        GeneratedField::EntryFunctionPayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entryFunctionPayload"));
                            }
                            payload__ = map.next_value::<::std::option::Option<_>>()?.map(multisig_transaction_payload::Payload::EntryFunctionPayload)
;
                        }
                    }
                }
                Ok(MultisigTransactionPayload {
                    r#type: r#type__.unwrap_or_default(),
                    payload: payload__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.MultisigTransactionPayload", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for multisig_transaction_payload::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::EntryFunctionPayload => "TYPE_ENTRY_FUNCTION_PAYLOAD",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for multisig_transaction_payload::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "TYPE_ENTRY_FUNCTION_PAYLOAD",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = multisig_transaction_payload::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(multisig_transaction_payload::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(multisig_transaction_payload::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(multisig_transaction_payload::Type::Unspecified),
                    "TYPE_ENTRY_FUNCTION_PAYLOAD" => Ok(multisig_transaction_payload::Type::EntryFunctionPayload),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ScriptPayload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.code.is_some() {
            len += 1;
        }
        if !self.type_arguments.is_empty() {
            len += 1;
        }
        if !self.arguments.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ScriptPayload", len)?;
        if let Some(v) = self.code.as_ref() {
            struct_ser.serialize_field("code", v)?;
        }
        if !self.type_arguments.is_empty() {
            struct_ser.serialize_field("typeArguments", &self.type_arguments)?;
        }
        if !self.arguments.is_empty() {
            struct_ser.serialize_field("arguments", &self.arguments)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ScriptPayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "code",
            "type_arguments",
            "typeArguments",
            "arguments",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Code,
            TypeArguments,
            Arguments,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "code" => Ok(GeneratedField::Code),
                            "typeArguments" | "type_arguments" => Ok(GeneratedField::TypeArguments),
                            "arguments" => Ok(GeneratedField::Arguments),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ScriptPayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ScriptPayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ScriptPayload, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut code__ = None;
                let mut type_arguments__ = None;
                let mut arguments__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Code => {
                            if code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("code"));
                            }
                            code__ = map.next_value()?;
                        }
                        GeneratedField::TypeArguments => {
                            if type_arguments__.is_some() {
                                return Err(serde::de::Error::duplicate_field("typeArguments"));
                            }
                            type_arguments__ = Some(map.next_value()?);
                        }
                        GeneratedField::Arguments => {
                            if arguments__.is_some() {
                                return Err(serde::de::Error::duplicate_field("arguments"));
                            }
                            arguments__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ScriptPayload {
                    code: code__,
                    type_arguments: type_arguments__.unwrap_or_default(),
                    arguments: arguments__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ScriptPayload", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ScriptWriteSet {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.execute_as.is_empty() {
            len += 1;
        }
        if self.script.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ScriptWriteSet", len)?;
        if !self.execute_as.is_empty() {
            struct_ser.serialize_field("executeAs", &self.execute_as)?;
        }
        if let Some(v) = self.script.as_ref() {
            struct_ser.serialize_field("script", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ScriptWriteSet {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "execute_as",
            "executeAs",
            "script",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ExecuteAs,
            Script,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "executeAs" | "execute_as" => Ok(GeneratedField::ExecuteAs),
                            "script" => Ok(GeneratedField::Script),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ScriptWriteSet;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ScriptWriteSet")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ScriptWriteSet, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut execute_as__ = None;
                let mut script__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ExecuteAs => {
                            if execute_as__.is_some() {
                                return Err(serde::de::Error::duplicate_field("executeAs"));
                            }
                            execute_as__ = Some(map.next_value()?);
                        }
                        GeneratedField::Script => {
                            if script__.is_some() {
                                return Err(serde::de::Error::duplicate_field("script"));
                            }
                            script__ = map.next_value()?;
                        }
                    }
                }
                Ok(ScriptWriteSet {
                    execute_as: execute_as__.unwrap_or_default(),
                    script: script__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ScriptWriteSet", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Secp256k1Ecdsa {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signature.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.Secp256k1Ecdsa", len)?;
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Secp256k1Ecdsa {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Secp256k1Ecdsa;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.Secp256k1Ecdsa")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Secp256k1Ecdsa, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Secp256k1Ecdsa {
                    signature: signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.Secp256k1Ecdsa", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Signature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if self.signature.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.Signature", len)?;
        if self.r#type != 0 {
            let v = signature::Type::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if let Some(v) = self.signature.as_ref() {
            match v {
                signature::Signature::Ed25519(v) => {
                    struct_ser.serialize_field("ed25519", v)?;
                }
                signature::Signature::MultiEd25519(v) => {
                    struct_ser.serialize_field("multiEd25519", v)?;
                }
                signature::Signature::MultiAgent(v) => {
                    struct_ser.serialize_field("multiAgent", v)?;
                }
                signature::Signature::FeePayer(v) => {
                    struct_ser.serialize_field("feePayer", v)?;
                }
                signature::Signature::SingleSender(v) => {
                    struct_ser.serialize_field("singleSender", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Signature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "ed25519",
            "multi_ed25519",
            "multiEd25519",
            "multi_agent",
            "multiAgent",
            "fee_payer",
            "feePayer",
            "single_sender",
            "singleSender",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Ed25519,
            MultiEd25519,
            MultiAgent,
            FeePayer,
            SingleSender,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "ed25519" => Ok(GeneratedField::Ed25519),
                            "multiEd25519" | "multi_ed25519" => Ok(GeneratedField::MultiEd25519),
                            "multiAgent" | "multi_agent" => Ok(GeneratedField::MultiAgent),
                            "feePayer" | "fee_payer" => Ok(GeneratedField::FeePayer),
                            "singleSender" | "single_sender" => Ok(GeneratedField::SingleSender),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Signature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.Signature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Signature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<signature::Type>()? as i32);
                        }
                        GeneratedField::Ed25519 => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ed25519"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(signature::Signature::Ed25519)
;
                        }
                        GeneratedField::MultiEd25519 => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiEd25519"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(signature::Signature::MultiEd25519)
;
                        }
                        GeneratedField::MultiAgent => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiAgent"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(signature::Signature::MultiAgent)
;
                        }
                        GeneratedField::FeePayer => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feePayer"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(signature::Signature::FeePayer)
;
                        }
                        GeneratedField::SingleSender => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("singleSender"));
                            }
                            signature__ = map.next_value::<::std::option::Option<_>>()?.map(signature::Signature::SingleSender)
;
                        }
                    }
                }
                Ok(Signature {
                    r#type: r#type__.unwrap_or_default(),
                    signature: signature__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.Signature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for signature::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::Ed25519 => "TYPE_ED25519",
            Self::MultiEd25519 => "TYPE_MULTI_ED25519",
            Self::MultiAgent => "TYPE_MULTI_AGENT",
            Self::FeePayer => "TYPE_FEE_PAYER",
            Self::SingleSender => "TYPE_SINGLE_SENDER",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for signature::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "TYPE_ED25519",
            "TYPE_MULTI_ED25519",
            "TYPE_MULTI_AGENT",
            "TYPE_FEE_PAYER",
            "TYPE_SINGLE_SENDER",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = signature::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(signature::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(signature::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(signature::Type::Unspecified),
                    "TYPE_ED25519" => Ok(signature::Type::Ed25519),
                    "TYPE_MULTI_ED25519" => Ok(signature::Type::MultiEd25519),
                    "TYPE_MULTI_AGENT" => Ok(signature::Type::MultiAgent),
                    "TYPE_FEE_PAYER" => Ok(signature::Type::FeePayer),
                    "TYPE_SINGLE_SENDER" => Ok(signature::Type::SingleSender),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for SingleKeySignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.public_key.is_some() {
            len += 1;
        }
        if self.signature.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.SingleKeySignature", len)?;
        if let Some(v) = self.public_key.as_ref() {
            struct_ser.serialize_field("publicKey", v)?;
        }
        if let Some(v) = self.signature.as_ref() {
            struct_ser.serialize_field("signature", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SingleKeySignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "public_key",
            "publicKey",
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PublicKey,
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "publicKey" | "public_key" => Ok(GeneratedField::PublicKey),
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SingleKeySignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.SingleKeySignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<SingleKeySignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut public_key__ = None;
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::PublicKey => {
                            if public_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKey"));
                            }
                            public_key__ = map.next_value()?;
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = map.next_value()?;
                        }
                    }
                }
                Ok(SingleKeySignature {
                    public_key: public_key__,
                    signature: signature__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.SingleKeySignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SingleSender {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.sender.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.SingleSender", len)?;
        if let Some(v) = self.sender.as_ref() {
            struct_ser.serialize_field("sender", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SingleSender {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sender",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Sender,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "sender" => Ok(GeneratedField::Sender),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SingleSender;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.SingleSender")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<SingleSender, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sender__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Sender => {
                            if sender__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sender"));
                            }
                            sender__ = map.next_value()?;
                        }
                    }
                }
                Ok(SingleSender {
                    sender: sender__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.SingleSender", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StateCheckpointTransaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("velor.transaction.v1.StateCheckpointTransaction", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StateCheckpointTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StateCheckpointTransaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.StateCheckpointTransaction")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<StateCheckpointTransaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(StateCheckpointTransaction {
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.StateCheckpointTransaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Transaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.version != 0 {
            len += 1;
        }
        if self.info.is_some() {
            len += 1;
        }
        if self.epoch != 0 {
            len += 1;
        }
        if self.block_height != 0 {
            len += 1;
        }
        if self.r#type != 0 {
            len += 1;
        }
        if self.size_info.is_some() {
            len += 1;
        }
        if self.txn_data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.Transaction", len)?;
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if let Some(v) = self.info.as_ref() {
            struct_ser.serialize_field("info", v)?;
        }
        if self.epoch != 0 {
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        if self.block_height != 0 {
            struct_ser.serialize_field("blockHeight", ToString::to_string(&self.block_height).as_str())?;
        }
        if self.r#type != 0 {
            let v = transaction::TransactionType::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if let Some(v) = self.size_info.as_ref() {
            struct_ser.serialize_field("sizeInfo", v)?;
        }
        if let Some(v) = self.txn_data.as_ref() {
            match v {
                transaction::TxnData::BlockMetadata(v) => {
                    struct_ser.serialize_field("blockMetadata", v)?;
                }
                transaction::TxnData::Genesis(v) => {
                    struct_ser.serialize_field("genesis", v)?;
                }
                transaction::TxnData::StateCheckpoint(v) => {
                    struct_ser.serialize_field("stateCheckpoint", v)?;
                }
                transaction::TxnData::User(v) => {
                    struct_ser.serialize_field("user", v)?;
                }
                transaction::TxnData::Validator(v) => {
                    struct_ser.serialize_field("validator", v)?;
                }
                transaction::TxnData::BlockEpilogue(v) => {
                    struct_ser.serialize_field("blockEpilogue", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Transaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "timestamp",
            "version",
            "info",
            "epoch",
            "block_height",
            "blockHeight",
            "type",
            "size_info",
            "sizeInfo",
            "block_metadata",
            "blockMetadata",
            "genesis",
            "state_checkpoint",
            "stateCheckpoint",
            "user",
            "validator",
            "block_epilogue",
            "blockEpilogue",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Timestamp,
            Version,
            Info,
            Epoch,
            BlockHeight,
            Type,
            SizeInfo,
            BlockMetadata,
            Genesis,
            StateCheckpoint,
            User,
            Validator,
            BlockEpilogue,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "version" => Ok(GeneratedField::Version),
                            "info" => Ok(GeneratedField::Info),
                            "epoch" => Ok(GeneratedField::Epoch),
                            "blockHeight" | "block_height" => Ok(GeneratedField::BlockHeight),
                            "type" => Ok(GeneratedField::Type),
                            "sizeInfo" | "size_info" => Ok(GeneratedField::SizeInfo),
                            "blockMetadata" | "block_metadata" => Ok(GeneratedField::BlockMetadata),
                            "genesis" => Ok(GeneratedField::Genesis),
                            "stateCheckpoint" | "state_checkpoint" => Ok(GeneratedField::StateCheckpoint),
                            "user" => Ok(GeneratedField::User),
                            "validator" => Ok(GeneratedField::Validator),
                            "blockEpilogue" | "block_epilogue" => Ok(GeneratedField::BlockEpilogue),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Transaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.Transaction")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Transaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut timestamp__ = None;
                let mut version__ = None;
                let mut info__ = None;
                let mut epoch__ = None;
                let mut block_height__ = None;
                let mut r#type__ = None;
                let mut size_info__ = None;
                let mut txn_data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Info => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("info"));
                            }
                            info__ = map.next_value()?;
                        }
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<transaction::TransactionType>()? as i32);
                        }
                        GeneratedField::SizeInfo => {
                            if size_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sizeInfo"));
                            }
                            size_info__ = map.next_value()?;
                        }
                        GeneratedField::BlockMetadata => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockMetadata"));
                            }
                            txn_data__ = map.next_value::<::std::option::Option<_>>()?.map(transaction::TxnData::BlockMetadata)
;
                        }
                        GeneratedField::Genesis => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genesis"));
                            }
                            txn_data__ = map.next_value::<::std::option::Option<_>>()?.map(transaction::TxnData::Genesis)
;
                        }
                        GeneratedField::StateCheckpoint => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateCheckpoint"));
                            }
                            txn_data__ = map.next_value::<::std::option::Option<_>>()?.map(transaction::TxnData::StateCheckpoint)
;
                        }
                        GeneratedField::User => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("user"));
                            }
                            txn_data__ = map.next_value::<::std::option::Option<_>>()?.map(transaction::TxnData::User)
;
                        }
                        GeneratedField::Validator => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validator"));
                            }
                            txn_data__ = map.next_value::<::std::option::Option<_>>()?.map(transaction::TxnData::Validator)
;
                        }
                        GeneratedField::BlockEpilogue => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockEpilogue"));
                            }
                            txn_data__ = map.next_value::<::std::option::Option<_>>()?.map(transaction::TxnData::BlockEpilogue)
;
                        }
                    }
                }
                Ok(Transaction {
                    timestamp: timestamp__,
                    version: version__.unwrap_or_default(),
                    info: info__,
                    epoch: epoch__.unwrap_or_default(),
                    block_height: block_height__.unwrap_or_default(),
                    r#type: r#type__.unwrap_or_default(),
                    size_info: size_info__,
                    txn_data: txn_data__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.Transaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction::TransactionType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TRANSACTION_TYPE_UNSPECIFIED",
            Self::Genesis => "TRANSACTION_TYPE_GENESIS",
            Self::BlockMetadata => "TRANSACTION_TYPE_BLOCK_METADATA",
            Self::StateCheckpoint => "TRANSACTION_TYPE_STATE_CHECKPOINT",
            Self::User => "TRANSACTION_TYPE_USER",
            Self::Validator => "TRANSACTION_TYPE_VALIDATOR",
            Self::BlockEpilogue => "TRANSACTION_TYPE_BLOCK_EPILOGUE",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for transaction::TransactionType {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TRANSACTION_TYPE_UNSPECIFIED",
            "TRANSACTION_TYPE_GENESIS",
            "TRANSACTION_TYPE_BLOCK_METADATA",
            "TRANSACTION_TYPE_STATE_CHECKPOINT",
            "TRANSACTION_TYPE_USER",
            "TRANSACTION_TYPE_VALIDATOR",
            "TRANSACTION_TYPE_BLOCK_EPILOGUE",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = transaction::TransactionType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(transaction::TransactionType::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(transaction::TransactionType::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TRANSACTION_TYPE_UNSPECIFIED" => Ok(transaction::TransactionType::Unspecified),
                    "TRANSACTION_TYPE_GENESIS" => Ok(transaction::TransactionType::Genesis),
                    "TRANSACTION_TYPE_BLOCK_METADATA" => Ok(transaction::TransactionType::BlockMetadata),
                    "TRANSACTION_TYPE_STATE_CHECKPOINT" => Ok(transaction::TransactionType::StateCheckpoint),
                    "TRANSACTION_TYPE_USER" => Ok(transaction::TransactionType::User),
                    "TRANSACTION_TYPE_VALIDATOR" => Ok(transaction::TransactionType::Validator),
                    "TRANSACTION_TYPE_BLOCK_EPILOGUE" => Ok(transaction::TransactionType::BlockEpilogue),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.hash.is_empty() {
            len += 1;
        }
        if !self.state_change_hash.is_empty() {
            len += 1;
        }
        if !self.event_root_hash.is_empty() {
            len += 1;
        }
        if self.state_checkpoint_hash.is_some() {
            len += 1;
        }
        if self.gas_used != 0 {
            len += 1;
        }
        if self.success {
            len += 1;
        }
        if !self.vm_status.is_empty() {
            len += 1;
        }
        if !self.accumulator_root_hash.is_empty() {
            len += 1;
        }
        if !self.changes.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.TransactionInfo", len)?;
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", pbjson::private::base64::encode(&self.hash).as_str())?;
        }
        if !self.state_change_hash.is_empty() {
            struct_ser.serialize_field("stateChangeHash", pbjson::private::base64::encode(&self.state_change_hash).as_str())?;
        }
        if !self.event_root_hash.is_empty() {
            struct_ser.serialize_field("eventRootHash", pbjson::private::base64::encode(&self.event_root_hash).as_str())?;
        }
        if let Some(v) = self.state_checkpoint_hash.as_ref() {
            struct_ser.serialize_field("stateCheckpointHash", pbjson::private::base64::encode(&v).as_str())?;
        }
        if self.gas_used != 0 {
            struct_ser.serialize_field("gasUsed", ToString::to_string(&self.gas_used).as_str())?;
        }
        if self.success {
            struct_ser.serialize_field("success", &self.success)?;
        }
        if !self.vm_status.is_empty() {
            struct_ser.serialize_field("vmStatus", &self.vm_status)?;
        }
        if !self.accumulator_root_hash.is_empty() {
            struct_ser.serialize_field("accumulatorRootHash", pbjson::private::base64::encode(&self.accumulator_root_hash).as_str())?;
        }
        if !self.changes.is_empty() {
            struct_ser.serialize_field("changes", &self.changes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "hash",
            "state_change_hash",
            "stateChangeHash",
            "event_root_hash",
            "eventRootHash",
            "state_checkpoint_hash",
            "stateCheckpointHash",
            "gas_used",
            "gasUsed",
            "success",
            "vm_status",
            "vmStatus",
            "accumulator_root_hash",
            "accumulatorRootHash",
            "changes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Hash,
            StateChangeHash,
            EventRootHash,
            StateCheckpointHash,
            GasUsed,
            Success,
            VmStatus,
            AccumulatorRootHash,
            Changes,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "hash" => Ok(GeneratedField::Hash),
                            "stateChangeHash" | "state_change_hash" => Ok(GeneratedField::StateChangeHash),
                            "eventRootHash" | "event_root_hash" => Ok(GeneratedField::EventRootHash),
                            "stateCheckpointHash" | "state_checkpoint_hash" => Ok(GeneratedField::StateCheckpointHash),
                            "gasUsed" | "gas_used" => Ok(GeneratedField::GasUsed),
                            "success" => Ok(GeneratedField::Success),
                            "vmStatus" | "vm_status" => Ok(GeneratedField::VmStatus),
                            "accumulatorRootHash" | "accumulator_root_hash" => Ok(GeneratedField::AccumulatorRootHash),
                            "changes" => Ok(GeneratedField::Changes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.TransactionInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut hash__ = None;
                let mut state_change_hash__ = None;
                let mut event_root_hash__ = None;
                let mut state_checkpoint_hash__ = None;
                let mut gas_used__ = None;
                let mut success__ = None;
                let mut vm_status__ = None;
                let mut accumulator_root_hash__ = None;
                let mut changes__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StateChangeHash => {
                            if state_change_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateChangeHash"));
                            }
                            state_change_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EventRootHash => {
                            if event_root_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("eventRootHash"));
                            }
                            event_root_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StateCheckpointHash => {
                            if state_checkpoint_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateCheckpointHash"));
                            }
                            state_checkpoint_hash__ =
                                map.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::GasUsed => {
                            if gas_used__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gasUsed"));
                            }
                            gas_used__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Success => {
                            if success__.is_some() {
                                return Err(serde::de::Error::duplicate_field("success"));
                            }
                            success__ = Some(map.next_value()?);
                        }
                        GeneratedField::VmStatus => {
                            if vm_status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("vmStatus"));
                            }
                            vm_status__ = Some(map.next_value()?);
                        }
                        GeneratedField::AccumulatorRootHash => {
                            if accumulator_root_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("accumulatorRootHash"));
                            }
                            accumulator_root_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Changes => {
                            if changes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("changes"));
                            }
                            changes__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(TransactionInfo {
                    hash: hash__.unwrap_or_default(),
                    state_change_hash: state_change_hash__.unwrap_or_default(),
                    event_root_hash: event_root_hash__.unwrap_or_default(),
                    state_checkpoint_hash: state_checkpoint_hash__,
                    gas_used: gas_used__.unwrap_or_default(),
                    success: success__.unwrap_or_default(),
                    vm_status: vm_status__.unwrap_or_default(),
                    accumulator_root_hash: accumulator_root_hash__.unwrap_or_default(),
                    changes: changes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.TransactionInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionPayload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if self.payload.is_some() {
            len += 1;
        }
        if self.extra_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.TransactionPayload", len)?;
        if self.r#type != 0 {
            let v = transaction_payload::Type::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if let Some(v) = self.payload.as_ref() {
            match v {
                transaction_payload::Payload::EntryFunctionPayload(v) => {
                    struct_ser.serialize_field("entryFunctionPayload", v)?;
                }
                transaction_payload::Payload::ScriptPayload(v) => {
                    struct_ser.serialize_field("scriptPayload", v)?;
                }
                transaction_payload::Payload::WriteSetPayload(v) => {
                    struct_ser.serialize_field("writeSetPayload", v)?;
                }
                transaction_payload::Payload::MultisigPayload(v) => {
                    struct_ser.serialize_field("multisigPayload", v)?;
                }
            }
        }
        if let Some(v) = self.extra_config.as_ref() {
            match v {
                transaction_payload::ExtraConfig::ExtraConfigV1(v) => {
                    struct_ser.serialize_field("extraConfigV1", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionPayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "entry_function_payload",
            "entryFunctionPayload",
            "script_payload",
            "scriptPayload",
            "write_set_payload",
            "writeSetPayload",
            "multisig_payload",
            "multisigPayload",
            "extra_config_v1",
            "extraConfigV1",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            EntryFunctionPayload,
            ScriptPayload,
            WriteSetPayload,
            MultisigPayload,
            ExtraConfigV1,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "entryFunctionPayload" | "entry_function_payload" => Ok(GeneratedField::EntryFunctionPayload),
                            "scriptPayload" | "script_payload" => Ok(GeneratedField::ScriptPayload),
                            "writeSetPayload" | "write_set_payload" => Ok(GeneratedField::WriteSetPayload),
                            "multisigPayload" | "multisig_payload" => Ok(GeneratedField::MultisigPayload),
                            "extraConfigV1" | "extra_config_v1" => Ok(GeneratedField::ExtraConfigV1),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionPayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.TransactionPayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionPayload, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut payload__ = None;
                let mut extra_config__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<transaction_payload::Type>()? as i32);
                        }
                        GeneratedField::EntryFunctionPayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entryFunctionPayload"));
                            }
                            payload__ = map.next_value::<::std::option::Option<_>>()?.map(transaction_payload::Payload::EntryFunctionPayload)
;
                        }
                        GeneratedField::ScriptPayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("scriptPayload"));
                            }
                            payload__ = map.next_value::<::std::option::Option<_>>()?.map(transaction_payload::Payload::ScriptPayload)
;
                        }
                        GeneratedField::WriteSetPayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeSetPayload"));
                            }
                            payload__ = map.next_value::<::std::option::Option<_>>()?.map(transaction_payload::Payload::WriteSetPayload)
;
                        }
                        GeneratedField::MultisigPayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multisigPayload"));
                            }
                            payload__ = map.next_value::<::std::option::Option<_>>()?.map(transaction_payload::Payload::MultisigPayload)
;
                        }
                        GeneratedField::ExtraConfigV1 => {
                            if extra_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("extraConfigV1"));
                            }
                            extra_config__ = map.next_value::<::std::option::Option<_>>()?.map(transaction_payload::ExtraConfig::ExtraConfigV1)
;
                        }
                    }
                }
                Ok(TransactionPayload {
                    r#type: r#type__.unwrap_or_default(),
                    payload: payload__,
                    extra_config: extra_config__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.TransactionPayload", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction_payload::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::EntryFunctionPayload => "TYPE_ENTRY_FUNCTION_PAYLOAD",
            Self::ScriptPayload => "TYPE_SCRIPT_PAYLOAD",
            Self::WriteSetPayload => "TYPE_WRITE_SET_PAYLOAD",
            Self::MultisigPayload => "TYPE_MULTISIG_PAYLOAD",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for transaction_payload::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "TYPE_ENTRY_FUNCTION_PAYLOAD",
            "TYPE_SCRIPT_PAYLOAD",
            "TYPE_WRITE_SET_PAYLOAD",
            "TYPE_MULTISIG_PAYLOAD",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = transaction_payload::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(transaction_payload::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(transaction_payload::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(transaction_payload::Type::Unspecified),
                    "TYPE_ENTRY_FUNCTION_PAYLOAD" => Ok(transaction_payload::Type::EntryFunctionPayload),
                    "TYPE_SCRIPT_PAYLOAD" => Ok(transaction_payload::Type::ScriptPayload),
                    "TYPE_WRITE_SET_PAYLOAD" => Ok(transaction_payload::Type::WriteSetPayload),
                    "TYPE_MULTISIG_PAYLOAD" => Ok(transaction_payload::Type::MultisigPayload),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionSizeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transaction_bytes != 0 {
            len += 1;
        }
        if !self.event_size_info.is_empty() {
            len += 1;
        }
        if !self.write_op_size_info.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.TransactionSizeInfo", len)?;
        if self.transaction_bytes != 0 {
            struct_ser.serialize_field("transactionBytes", &self.transaction_bytes)?;
        }
        if !self.event_size_info.is_empty() {
            struct_ser.serialize_field("eventSizeInfo", &self.event_size_info)?;
        }
        if !self.write_op_size_info.is_empty() {
            struct_ser.serialize_field("writeOpSizeInfo", &self.write_op_size_info)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionSizeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction_bytes",
            "transactionBytes",
            "event_size_info",
            "eventSizeInfo",
            "write_op_size_info",
            "writeOpSizeInfo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionBytes,
            EventSizeInfo,
            WriteOpSizeInfo,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "transactionBytes" | "transaction_bytes" => Ok(GeneratedField::TransactionBytes),
                            "eventSizeInfo" | "event_size_info" => Ok(GeneratedField::EventSizeInfo),
                            "writeOpSizeInfo" | "write_op_size_info" => Ok(GeneratedField::WriteOpSizeInfo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionSizeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.TransactionSizeInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionSizeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transaction_bytes__ = None;
                let mut event_size_info__ = None;
                let mut write_op_size_info__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TransactionBytes => {
                            if transaction_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionBytes"));
                            }
                            transaction_bytes__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EventSizeInfo => {
                            if event_size_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("eventSizeInfo"));
                            }
                            event_size_info__ = Some(map.next_value()?);
                        }
                        GeneratedField::WriteOpSizeInfo => {
                            if write_op_size_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeOpSizeInfo"));
                            }
                            write_op_size_info__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(TransactionSizeInfo {
                    transaction_bytes: transaction_bytes__.unwrap_or_default(),
                    event_size_info: event_size_info__.unwrap_or_default(),
                    write_op_size_info: write_op_size_info__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.TransactionSizeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UserTransaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.request.is_some() {
            len += 1;
        }
        if !self.events.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.UserTransaction", len)?;
        if let Some(v) = self.request.as_ref() {
            struct_ser.serialize_field("request", v)?;
        }
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UserTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "request",
            "events",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Request,
            Events,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "request" => Ok(GeneratedField::Request),
                            "events" => Ok(GeneratedField::Events),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UserTransaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.UserTransaction")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<UserTransaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut request__ = None;
                let mut events__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Request => {
                            if request__.is_some() {
                                return Err(serde::de::Error::duplicate_field("request"));
                            }
                            request__ = map.next_value()?;
                        }
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(UserTransaction {
                    request: request__,
                    events: events__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.UserTransaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UserTransactionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.sender.is_empty() {
            len += 1;
        }
        if self.sequence_number != 0 {
            len += 1;
        }
        if self.max_gas_amount != 0 {
            len += 1;
        }
        if self.gas_unit_price != 0 {
            len += 1;
        }
        if self.expiration_timestamp_secs.is_some() {
            len += 1;
        }
        if self.payload.is_some() {
            len += 1;
        }
        if self.signature.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.UserTransactionRequest", len)?;
        if !self.sender.is_empty() {
            struct_ser.serialize_field("sender", &self.sender)?;
        }
        if self.sequence_number != 0 {
            struct_ser.serialize_field("sequenceNumber", ToString::to_string(&self.sequence_number).as_str())?;
        }
        if self.max_gas_amount != 0 {
            struct_ser.serialize_field("maxGasAmount", ToString::to_string(&self.max_gas_amount).as_str())?;
        }
        if self.gas_unit_price != 0 {
            struct_ser.serialize_field("gasUnitPrice", ToString::to_string(&self.gas_unit_price).as_str())?;
        }
        if let Some(v) = self.expiration_timestamp_secs.as_ref() {
            struct_ser.serialize_field("expirationTimestampSecs", v)?;
        }
        if let Some(v) = self.payload.as_ref() {
            struct_ser.serialize_field("payload", v)?;
        }
        if let Some(v) = self.signature.as_ref() {
            struct_ser.serialize_field("signature", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UserTransactionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sender",
            "sequence_number",
            "sequenceNumber",
            "max_gas_amount",
            "maxGasAmount",
            "gas_unit_price",
            "gasUnitPrice",
            "expiration_timestamp_secs",
            "expirationTimestampSecs",
            "payload",
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Sender,
            SequenceNumber,
            MaxGasAmount,
            GasUnitPrice,
            ExpirationTimestampSecs,
            Payload,
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "sender" => Ok(GeneratedField::Sender),
                            "sequenceNumber" | "sequence_number" => Ok(GeneratedField::SequenceNumber),
                            "maxGasAmount" | "max_gas_amount" => Ok(GeneratedField::MaxGasAmount),
                            "gasUnitPrice" | "gas_unit_price" => Ok(GeneratedField::GasUnitPrice),
                            "expirationTimestampSecs" | "expiration_timestamp_secs" => Ok(GeneratedField::ExpirationTimestampSecs),
                            "payload" => Ok(GeneratedField::Payload),
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UserTransactionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.UserTransactionRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<UserTransactionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sender__ = None;
                let mut sequence_number__ = None;
                let mut max_gas_amount__ = None;
                let mut gas_unit_price__ = None;
                let mut expiration_timestamp_secs__ = None;
                let mut payload__ = None;
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Sender => {
                            if sender__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sender"));
                            }
                            sender__ = Some(map.next_value()?);
                        }
                        GeneratedField::SequenceNumber => {
                            if sequence_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequenceNumber"));
                            }
                            sequence_number__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MaxGasAmount => {
                            if max_gas_amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxGasAmount"));
                            }
                            max_gas_amount__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::GasUnitPrice => {
                            if gas_unit_price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gasUnitPrice"));
                            }
                            gas_unit_price__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ExpirationTimestampSecs => {
                            if expiration_timestamp_secs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("expirationTimestampSecs"));
                            }
                            expiration_timestamp_secs__ = map.next_value()?;
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = map.next_value()?;
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = map.next_value()?;
                        }
                    }
                }
                Ok(UserTransactionRequest {
                    sender: sender__.unwrap_or_default(),
                    sequence_number: sequence_number__.unwrap_or_default(),
                    max_gas_amount: max_gas_amount__.unwrap_or_default(),
                    gas_unit_price: gas_unit_price__.unwrap_or_default(),
                    expiration_timestamp_secs: expiration_timestamp_secs__,
                    payload: payload__,
                    signature: signature__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.UserTransactionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ValidatorTransaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.events.is_empty() {
            len += 1;
        }
        if self.validator_transaction_type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction", len)?;
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        if let Some(v) = self.validator_transaction_type.as_ref() {
            match v {
                validator_transaction::ValidatorTransactionType::ObservedJwkUpdate(v) => {
                    struct_ser.serialize_field("observedJwkUpdate", v)?;
                }
                validator_transaction::ValidatorTransactionType::DkgUpdate(v) => {
                    struct_ser.serialize_field("dkgUpdate", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ValidatorTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "events",
            "observed_jwk_update",
            "observedJwkUpdate",
            "dkg_update",
            "dkgUpdate",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Events,
            ObservedJwkUpdate,
            DkgUpdate,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "events" => Ok(GeneratedField::Events),
                            "observedJwkUpdate" | "observed_jwk_update" => Ok(GeneratedField::ObservedJwkUpdate),
                            "dkgUpdate" | "dkg_update" => Ok(GeneratedField::DkgUpdate),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ValidatorTransaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ValidatorTransaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut events__ = None;
                let mut validator_transaction_type__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                        GeneratedField::ObservedJwkUpdate => {
                            if validator_transaction_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("observedJwkUpdate"));
                            }
                            validator_transaction_type__ = map.next_value::<::std::option::Option<_>>()?.map(validator_transaction::ValidatorTransactionType::ObservedJwkUpdate)
;
                        }
                        GeneratedField::DkgUpdate => {
                            if validator_transaction_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dkgUpdate"));
                            }
                            validator_transaction_type__ = map.next_value::<::std::option::Option<_>>()?.map(validator_transaction::ValidatorTransactionType::DkgUpdate)
;
                        }
                    }
                }
                Ok(ValidatorTransaction {
                    events: events__.unwrap_or_default(),
                    validator_transaction_type: validator_transaction_type__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::DkgUpdate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.dkg_transcript.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.DkgUpdate", len)?;
        if let Some(v) = self.dkg_transcript.as_ref() {
            struct_ser.serialize_field("dkgTranscript", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::DkgUpdate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "dkg_transcript",
            "dkgTranscript",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DkgTranscript,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "dkgTranscript" | "dkg_transcript" => Ok(GeneratedField::DkgTranscript),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::DkgUpdate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.DkgUpdate")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::DkgUpdate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut dkg_transcript__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::DkgTranscript => {
                            if dkg_transcript__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dkgTranscript"));
                            }
                            dkg_transcript__ = map.next_value()?;
                        }
                    }
                }
                Ok(validator_transaction::DkgUpdate {
                    dkg_transcript: dkg_transcript__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.DkgUpdate", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::dkg_update::DkgTranscript {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.epoch != 0 {
            len += 1;
        }
        if !self.author.is_empty() {
            len += 1;
        }
        if !self.payload.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.DkgUpdate.DkgTranscript", len)?;
        if self.epoch != 0 {
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        if !self.author.is_empty() {
            struct_ser.serialize_field("author", &self.author)?;
        }
        if !self.payload.is_empty() {
            struct_ser.serialize_field("payload", pbjson::private::base64::encode(&self.payload).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::dkg_update::DkgTranscript {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "epoch",
            "author",
            "payload",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Epoch,
            Author,
            Payload,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "epoch" => Ok(GeneratedField::Epoch),
                            "author" => Ok(GeneratedField::Author),
                            "payload" => Ok(GeneratedField::Payload),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::dkg_update::DkgTranscript;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.DkgUpdate.DkgTranscript")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::dkg_update::DkgTranscript, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut epoch__ = None;
                let mut author__ = None;
                let mut payload__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Author => {
                            if author__.is_some() {
                                return Err(serde::de::Error::duplicate_field("author"));
                            }
                            author__ = Some(map.next_value()?);
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(validator_transaction::dkg_update::DkgTranscript {
                    epoch: epoch__.unwrap_or_default(),
                    author: author__.unwrap_or_default(),
                    payload: payload__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.DkgUpdate.DkgTranscript", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::ObservedJwkUpdate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.quorum_certified_update.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate", len)?;
        if let Some(v) = self.quorum_certified_update.as_ref() {
            struct_ser.serialize_field("quorumCertifiedUpdate", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::ObservedJwkUpdate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "quorum_certified_update",
            "quorumCertifiedUpdate",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            QuorumCertifiedUpdate,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "quorumCertifiedUpdate" | "quorum_certified_update" => Ok(GeneratedField::QuorumCertifiedUpdate),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::ObservedJwkUpdate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::ObservedJwkUpdate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut quorum_certified_update__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::QuorumCertifiedUpdate => {
                            if quorum_certified_update__.is_some() {
                                return Err(serde::de::Error::duplicate_field("quorumCertifiedUpdate"));
                            }
                            quorum_certified_update__ = map.next_value()?;
                        }
                    }
                }
                Ok(validator_transaction::ObservedJwkUpdate {
                    quorum_certified_update: quorum_certified_update__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::observed_jwk_update::ExportedAggregateSignature {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signer_indices.is_empty() {
            len += 1;
        }
        if !self.sig.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedAggregateSignature", len)?;
        if !self.signer_indices.is_empty() {
            struct_ser.serialize_field("signerIndices", &self.signer_indices.iter().map(ToString::to_string).collect::<Vec<_>>())?;
        }
        if !self.sig.is_empty() {
            struct_ser.serialize_field("sig", pbjson::private::base64::encode(&self.sig).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::observed_jwk_update::ExportedAggregateSignature {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signer_indices",
            "signerIndices",
            "sig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SignerIndices,
            Sig,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "signerIndices" | "signer_indices" => Ok(GeneratedField::SignerIndices),
                            "sig" => Ok(GeneratedField::Sig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::observed_jwk_update::ExportedAggregateSignature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedAggregateSignature")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::observed_jwk_update::ExportedAggregateSignature, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signer_indices__ = None;
                let mut sig__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::SignerIndices => {
                            if signer_indices__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signerIndices"));
                            }
                            signer_indices__ =
                                Some(map.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::Sig => {
                            if sig__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sig"));
                            }
                            sig__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(validator_transaction::observed_jwk_update::ExportedAggregateSignature {
                    signer_indices: signer_indices__.unwrap_or_default(),
                    sig: sig__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedAggregateSignature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::observed_jwk_update::ExportedProviderJwKs {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.issuer.is_empty() {
            len += 1;
        }
        if self.version != 0 {
            len += 1;
        }
        if !self.jwks.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs", len)?;
        if !self.issuer.is_empty() {
            struct_ser.serialize_field("issuer", &self.issuer)?;
        }
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if !self.jwks.is_empty() {
            struct_ser.serialize_field("jwks", &self.jwks)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::observed_jwk_update::ExportedProviderJwKs {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "issuer",
            "version",
            "jwks",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Issuer,
            Version,
            Jwks,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "issuer" => Ok(GeneratedField::Issuer),
                            "version" => Ok(GeneratedField::Version),
                            "jwks" => Ok(GeneratedField::Jwks),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::observed_jwk_update::ExportedProviderJwKs;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::observed_jwk_update::ExportedProviderJwKs, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut issuer__ = None;
                let mut version__ = None;
                let mut jwks__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Issuer => {
                            if issuer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("issuer"));
                            }
                            issuer__ = Some(map.next_value()?);
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Jwks => {
                            if jwks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("jwks"));
                            }
                            jwks__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(validator_transaction::observed_jwk_update::ExportedProviderJwKs {
                    issuer: issuer__.unwrap_or_default(),
                    version: version__.unwrap_or_default(),
                    jwks: jwks__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::observed_jwk_update::exported_provider_jw_ks::Jwk {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.jwk_type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK", len)?;
        if let Some(v) = self.jwk_type.as_ref() {
            match v {
                validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::JwkType::UnsupportedJwk(v) => {
                    struct_ser.serialize_field("unsupportedJwk", v)?;
                }
                validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::JwkType::Rsa(v) => {
                    struct_ser.serialize_field("rsa", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::observed_jwk_update::exported_provider_jw_ks::Jwk {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "unsupported_jwk",
            "unsupportedJwk",
            "rsa",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            UnsupportedJwk,
            Rsa,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "unsupportedJwk" | "unsupported_jwk" => Ok(GeneratedField::UnsupportedJwk),
                            "rsa" => Ok(GeneratedField::Rsa),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::observed_jwk_update::exported_provider_jw_ks::Jwk;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::observed_jwk_update::exported_provider_jw_ks::Jwk, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut jwk_type__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::UnsupportedJwk => {
                            if jwk_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("unsupportedJwk"));
                            }
                            jwk_type__ = map.next_value::<::std::option::Option<_>>()?.map(validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::JwkType::UnsupportedJwk)
;
                        }
                        GeneratedField::Rsa => {
                            if jwk_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rsa"));
                            }
                            jwk_type__ = map.next_value::<::std::option::Option<_>>()?.map(validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::JwkType::Rsa)
;
                        }
                    }
                }
                Ok(validator_transaction::observed_jwk_update::exported_provider_jw_ks::Jwk {
                    jwk_type: jwk_type__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::Rsa {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.kid.is_empty() {
            len += 1;
        }
        if !self.kty.is_empty() {
            len += 1;
        }
        if !self.alg.is_empty() {
            len += 1;
        }
        if !self.e.is_empty() {
            len += 1;
        }
        if !self.n.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.RSA", len)?;
        if !self.kid.is_empty() {
            struct_ser.serialize_field("kid", &self.kid)?;
        }
        if !self.kty.is_empty() {
            struct_ser.serialize_field("kty", &self.kty)?;
        }
        if !self.alg.is_empty() {
            struct_ser.serialize_field("alg", &self.alg)?;
        }
        if !self.e.is_empty() {
            struct_ser.serialize_field("e", &self.e)?;
        }
        if !self.n.is_empty() {
            struct_ser.serialize_field("n", &self.n)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::Rsa {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "kid",
            "kty",
            "alg",
            "e",
            "n",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Kid,
            Kty,
            Alg,
            E,
            N,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "kid" => Ok(GeneratedField::Kid),
                            "kty" => Ok(GeneratedField::Kty),
                            "alg" => Ok(GeneratedField::Alg),
                            "e" => Ok(GeneratedField::E),
                            "n" => Ok(GeneratedField::N),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::Rsa;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.RSA")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::Rsa, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut kid__ = None;
                let mut kty__ = None;
                let mut alg__ = None;
                let mut e__ = None;
                let mut n__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Kid => {
                            if kid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kid"));
                            }
                            kid__ = Some(map.next_value()?);
                        }
                        GeneratedField::Kty => {
                            if kty__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kty"));
                            }
                            kty__ = Some(map.next_value()?);
                        }
                        GeneratedField::Alg => {
                            if alg__.is_some() {
                                return Err(serde::de::Error::duplicate_field("alg"));
                            }
                            alg__ = Some(map.next_value()?);
                        }
                        GeneratedField::E => {
                            if e__.is_some() {
                                return Err(serde::de::Error::duplicate_field("e"));
                            }
                            e__ = Some(map.next_value()?);
                        }
                        GeneratedField::N => {
                            if n__.is_some() {
                                return Err(serde::de::Error::duplicate_field("n"));
                            }
                            n__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::Rsa {
                    kid: kid__.unwrap_or_default(),
                    kty: kty__.unwrap_or_default(),
                    alg: alg__.unwrap_or_default(),
                    e: e__.unwrap_or_default(),
                    n: n__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.RSA", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::UnsupportedJwk {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if !self.payload.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.UnsupportedJWK", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", pbjson::private::base64::encode(&self.id).as_str())?;
        }
        if !self.payload.is_empty() {
            struct_ser.serialize_field("payload", pbjson::private::base64::encode(&self.payload).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::UnsupportedJwk {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "payload",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Payload,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "payload" => Ok(GeneratedField::Payload),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::UnsupportedJwk;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.UnsupportedJWK")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::UnsupportedJwk, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut payload__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(validator_transaction::observed_jwk_update::exported_provider_jw_ks::jwk::UnsupportedJwk {
                    id: id__.unwrap_or_default(),
                    payload: payload__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.ExportedProviderJWKs.JWK.UnsupportedJWK", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for validator_transaction::observed_jwk_update::QuorumCertifiedUpdate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.update.is_some() {
            len += 1;
        }
        if self.multi_sig.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.QuorumCertifiedUpdate", len)?;
        if let Some(v) = self.update.as_ref() {
            struct_ser.serialize_field("update", v)?;
        }
        if let Some(v) = self.multi_sig.as_ref() {
            struct_ser.serialize_field("multiSig", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for validator_transaction::observed_jwk_update::QuorumCertifiedUpdate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "update",
            "multi_sig",
            "multiSig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Update,
            MultiSig,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "update" => Ok(GeneratedField::Update),
                            "multiSig" | "multi_sig" => Ok(GeneratedField::MultiSig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = validator_transaction::observed_jwk_update::QuorumCertifiedUpdate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.QuorumCertifiedUpdate")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<validator_transaction::observed_jwk_update::QuorumCertifiedUpdate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut update__ = None;
                let mut multi_sig__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Update => {
                            if update__.is_some() {
                                return Err(serde::de::Error::duplicate_field("update"));
                            }
                            update__ = map.next_value()?;
                        }
                        GeneratedField::MultiSig => {
                            if multi_sig__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiSig"));
                            }
                            multi_sig__ = map.next_value()?;
                        }
                    }
                }
                Ok(validator_transaction::observed_jwk_update::QuorumCertifiedUpdate {
                    update: update__,
                    multi_sig: multi_sig__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.ValidatorTransaction.ObservedJwkUpdate.QuorumCertifiedUpdate", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WebAuthn {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signature.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WebAuthn", len)?;
        if !self.signature.is_empty() {
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WebAuthn {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Signature,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "signature" => Ok(GeneratedField::Signature),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WebAuthn;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WebAuthn")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WebAuthn, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(WebAuthn {
                    signature: signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WebAuthn", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WriteModule {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.address.is_empty() {
            len += 1;
        }
        if !self.state_key_hash.is_empty() {
            len += 1;
        }
        if self.data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WriteModule", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field("stateKeyHash", pbjson::private::base64::encode(&self.state_key_hash).as_str())?;
        }
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteModule {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "state_key_hash",
            "stateKeyHash",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            StateKeyHash,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "stateKeyHash" | "state_key_hash" => Ok(GeneratedField::StateKeyHash),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteModule;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WriteModule")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteModule, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut state_key_hash__ = None;
                let mut data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        }
                        GeneratedField::StateKeyHash => {
                            if state_key_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateKeyHash"));
                            }
                            state_key_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map.next_value()?;
                        }
                    }
                }
                Ok(WriteModule {
                    address: address__.unwrap_or_default(),
                    state_key_hash: state_key_hash__.unwrap_or_default(),
                    data: data__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WriteModule", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WriteOpSizeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.key_bytes != 0 {
            len += 1;
        }
        if self.value_bytes != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WriteOpSizeInfo", len)?;
        if self.key_bytes != 0 {
            struct_ser.serialize_field("keyBytes", &self.key_bytes)?;
        }
        if self.value_bytes != 0 {
            struct_ser.serialize_field("valueBytes", &self.value_bytes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteOpSizeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key_bytes",
            "keyBytes",
            "value_bytes",
            "valueBytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            KeyBytes,
            ValueBytes,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "keyBytes" | "key_bytes" => Ok(GeneratedField::KeyBytes),
                            "valueBytes" | "value_bytes" => Ok(GeneratedField::ValueBytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteOpSizeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WriteOpSizeInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteOpSizeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key_bytes__ = None;
                let mut value_bytes__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::KeyBytes => {
                            if key_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("keyBytes"));
                            }
                            key_bytes__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ValueBytes => {
                            if value_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("valueBytes"));
                            }
                            value_bytes__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(WriteOpSizeInfo {
                    key_bytes: key_bytes__.unwrap_or_default(),
                    value_bytes: value_bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WriteOpSizeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WriteResource {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.address.is_empty() {
            len += 1;
        }
        if !self.state_key_hash.is_empty() {
            len += 1;
        }
        if self.r#type.is_some() {
            len += 1;
        }
        if !self.type_str.is_empty() {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WriteResource", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field("stateKeyHash", pbjson::private::base64::encode(&self.state_key_hash).as_str())?;
        }
        if let Some(v) = self.r#type.as_ref() {
            struct_ser.serialize_field("type", v)?;
        }
        if !self.type_str.is_empty() {
            struct_ser.serialize_field("typeStr", &self.type_str)?;
        }
        if !self.data.is_empty() {
            struct_ser.serialize_field("data", &self.data)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteResource {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "state_key_hash",
            "stateKeyHash",
            "type",
            "type_str",
            "typeStr",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            StateKeyHash,
            Type,
            TypeStr,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "stateKeyHash" | "state_key_hash" => Ok(GeneratedField::StateKeyHash),
                            "type" => Ok(GeneratedField::Type),
                            "typeStr" | "type_str" => Ok(GeneratedField::TypeStr),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteResource;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WriteResource")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteResource, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut state_key_hash__ = None;
                let mut r#type__ = None;
                let mut type_str__ = None;
                let mut data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        }
                        GeneratedField::StateKeyHash => {
                            if state_key_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateKeyHash"));
                            }
                            state_key_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = map.next_value()?;
                        }
                        GeneratedField::TypeStr => {
                            if type_str__.is_some() {
                                return Err(serde::de::Error::duplicate_field("typeStr"));
                            }
                            type_str__ = Some(map.next_value()?);
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(WriteResource {
                    address: address__.unwrap_or_default(),
                    state_key_hash: state_key_hash__.unwrap_or_default(),
                    r#type: r#type__,
                    type_str: type_str__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WriteResource", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WriteSet {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.write_set_type != 0 {
            len += 1;
        }
        if self.write_set.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WriteSet", len)?;
        if self.write_set_type != 0 {
            let v = write_set::WriteSetType::from_i32(self.write_set_type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.write_set_type)))?;
            struct_ser.serialize_field("writeSetType", &v)?;
        }
        if let Some(v) = self.write_set.as_ref() {
            match v {
                write_set::WriteSet::ScriptWriteSet(v) => {
                    struct_ser.serialize_field("scriptWriteSet", v)?;
                }
                write_set::WriteSet::DirectWriteSet(v) => {
                    struct_ser.serialize_field("directWriteSet", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteSet {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "write_set_type",
            "writeSetType",
            "script_write_set",
            "scriptWriteSet",
            "direct_write_set",
            "directWriteSet",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            WriteSetType,
            ScriptWriteSet,
            DirectWriteSet,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "writeSetType" | "write_set_type" => Ok(GeneratedField::WriteSetType),
                            "scriptWriteSet" | "script_write_set" => Ok(GeneratedField::ScriptWriteSet),
                            "directWriteSet" | "direct_write_set" => Ok(GeneratedField::DirectWriteSet),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteSet;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WriteSet")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteSet, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut write_set_type__ = None;
                let mut write_set__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::WriteSetType => {
                            if write_set_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeSetType"));
                            }
                            write_set_type__ = Some(map.next_value::<write_set::WriteSetType>()? as i32);
                        }
                        GeneratedField::ScriptWriteSet => {
                            if write_set__.is_some() {
                                return Err(serde::de::Error::duplicate_field("scriptWriteSet"));
                            }
                            write_set__ = map.next_value::<::std::option::Option<_>>()?.map(write_set::WriteSet::ScriptWriteSet)
;
                        }
                        GeneratedField::DirectWriteSet => {
                            if write_set__.is_some() {
                                return Err(serde::de::Error::duplicate_field("directWriteSet"));
                            }
                            write_set__ = map.next_value::<::std::option::Option<_>>()?.map(write_set::WriteSet::DirectWriteSet)
;
                        }
                    }
                }
                Ok(WriteSet {
                    write_set_type: write_set_type__.unwrap_or_default(),
                    write_set: write_set__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WriteSet", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for write_set::WriteSetType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "WRITE_SET_TYPE_UNSPECIFIED",
            Self::ScriptWriteSet => "WRITE_SET_TYPE_SCRIPT_WRITE_SET",
            Self::DirectWriteSet => "WRITE_SET_TYPE_DIRECT_WRITE_SET",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for write_set::WriteSetType {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "WRITE_SET_TYPE_UNSPECIFIED",
            "WRITE_SET_TYPE_SCRIPT_WRITE_SET",
            "WRITE_SET_TYPE_DIRECT_WRITE_SET",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = write_set::WriteSetType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(write_set::WriteSetType::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(write_set::WriteSetType::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "WRITE_SET_TYPE_UNSPECIFIED" => Ok(write_set::WriteSetType::Unspecified),
                    "WRITE_SET_TYPE_SCRIPT_WRITE_SET" => Ok(write_set::WriteSetType::ScriptWriteSet),
                    "WRITE_SET_TYPE_DIRECT_WRITE_SET" => Ok(write_set::WriteSetType::DirectWriteSet),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for WriteSetChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if self.change.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WriteSetChange", len)?;
        if self.r#type != 0 {
            let v = write_set_change::Type::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if let Some(v) = self.change.as_ref() {
            match v {
                write_set_change::Change::DeleteModule(v) => {
                    struct_ser.serialize_field("deleteModule", v)?;
                }
                write_set_change::Change::DeleteResource(v) => {
                    struct_ser.serialize_field("deleteResource", v)?;
                }
                write_set_change::Change::DeleteTableItem(v) => {
                    struct_ser.serialize_field("deleteTableItem", v)?;
                }
                write_set_change::Change::WriteModule(v) => {
                    struct_ser.serialize_field("writeModule", v)?;
                }
                write_set_change::Change::WriteResource(v) => {
                    struct_ser.serialize_field("writeResource", v)?;
                }
                write_set_change::Change::WriteTableItem(v) => {
                    struct_ser.serialize_field("writeTableItem", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteSetChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "delete_module",
            "deleteModule",
            "delete_resource",
            "deleteResource",
            "delete_table_item",
            "deleteTableItem",
            "write_module",
            "writeModule",
            "write_resource",
            "writeResource",
            "write_table_item",
            "writeTableItem",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            DeleteModule,
            DeleteResource,
            DeleteTableItem,
            WriteModule,
            WriteResource,
            WriteTableItem,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "deleteModule" | "delete_module" => Ok(GeneratedField::DeleteModule),
                            "deleteResource" | "delete_resource" => Ok(GeneratedField::DeleteResource),
                            "deleteTableItem" | "delete_table_item" => Ok(GeneratedField::DeleteTableItem),
                            "writeModule" | "write_module" => Ok(GeneratedField::WriteModule),
                            "writeResource" | "write_resource" => Ok(GeneratedField::WriteResource),
                            "writeTableItem" | "write_table_item" => Ok(GeneratedField::WriteTableItem),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteSetChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WriteSetChange")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteSetChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut change__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<write_set_change::Type>()? as i32);
                        }
                        GeneratedField::DeleteModule => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deleteModule"));
                            }
                            change__ = map.next_value::<::std::option::Option<_>>()?.map(write_set_change::Change::DeleteModule)
;
                        }
                        GeneratedField::DeleteResource => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deleteResource"));
                            }
                            change__ = map.next_value::<::std::option::Option<_>>()?.map(write_set_change::Change::DeleteResource)
;
                        }
                        GeneratedField::DeleteTableItem => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deleteTableItem"));
                            }
                            change__ = map.next_value::<::std::option::Option<_>>()?.map(write_set_change::Change::DeleteTableItem)
;
                        }
                        GeneratedField::WriteModule => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeModule"));
                            }
                            change__ = map.next_value::<::std::option::Option<_>>()?.map(write_set_change::Change::WriteModule)
;
                        }
                        GeneratedField::WriteResource => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeResource"));
                            }
                            change__ = map.next_value::<::std::option::Option<_>>()?.map(write_set_change::Change::WriteResource)
;
                        }
                        GeneratedField::WriteTableItem => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeTableItem"));
                            }
                            change__ = map.next_value::<::std::option::Option<_>>()?.map(write_set_change::Change::WriteTableItem)
;
                        }
                    }
                }
                Ok(WriteSetChange {
                    r#type: r#type__.unwrap_or_default(),
                    change: change__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WriteSetChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for write_set_change::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::DeleteModule => "TYPE_DELETE_MODULE",
            Self::DeleteResource => "TYPE_DELETE_RESOURCE",
            Self::DeleteTableItem => "TYPE_DELETE_TABLE_ITEM",
            Self::WriteModule => "TYPE_WRITE_MODULE",
            Self::WriteResource => "TYPE_WRITE_RESOURCE",
            Self::WriteTableItem => "TYPE_WRITE_TABLE_ITEM",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for write_set_change::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "TYPE_DELETE_MODULE",
            "TYPE_DELETE_RESOURCE",
            "TYPE_DELETE_TABLE_ITEM",
            "TYPE_WRITE_MODULE",
            "TYPE_WRITE_RESOURCE",
            "TYPE_WRITE_TABLE_ITEM",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = write_set_change::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(write_set_change::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use std::convert::TryFrom;
                i32::try_from(v)
                    .ok()
                    .and_then(write_set_change::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(write_set_change::Type::Unspecified),
                    "TYPE_DELETE_MODULE" => Ok(write_set_change::Type::DeleteModule),
                    "TYPE_DELETE_RESOURCE" => Ok(write_set_change::Type::DeleteResource),
                    "TYPE_DELETE_TABLE_ITEM" => Ok(write_set_change::Type::DeleteTableItem),
                    "TYPE_WRITE_MODULE" => Ok(write_set_change::Type::WriteModule),
                    "TYPE_WRITE_RESOURCE" => Ok(write_set_change::Type::WriteResource),
                    "TYPE_WRITE_TABLE_ITEM" => Ok(write_set_change::Type::WriteTableItem),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for WriteSetPayload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.write_set.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WriteSetPayload", len)?;
        if let Some(v) = self.write_set.as_ref() {
            struct_ser.serialize_field("writeSet", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteSetPayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "write_set",
            "writeSet",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            WriteSet,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "writeSet" | "write_set" => Ok(GeneratedField::WriteSet),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteSetPayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WriteSetPayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteSetPayload, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut write_set__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::WriteSet => {
                            if write_set__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeSet"));
                            }
                            write_set__ = map.next_value()?;
                        }
                    }
                }
                Ok(WriteSetPayload {
                    write_set: write_set__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WriteSetPayload", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WriteTableData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        if !self.key_type.is_empty() {
            len += 1;
        }
        if !self.value.is_empty() {
            len += 1;
        }
        if !self.value_type.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WriteTableData", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if !self.key_type.is_empty() {
            struct_ser.serialize_field("keyType", &self.key_type)?;
        }
        if !self.value.is_empty() {
            struct_ser.serialize_field("value", &self.value)?;
        }
        if !self.value_type.is_empty() {
            struct_ser.serialize_field("valueType", &self.value_type)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteTableData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "key_type",
            "keyType",
            "value",
            "value_type",
            "valueType",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            KeyType,
            Value,
            ValueType,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "keyType" | "key_type" => Ok(GeneratedField::KeyType),
                            "value" => Ok(GeneratedField::Value),
                            "valueType" | "value_type" => Ok(GeneratedField::ValueType),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteTableData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WriteTableData")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteTableData, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut key_type__ = None;
                let mut value__ = None;
                let mut value_type__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map.next_value()?);
                        }
                        GeneratedField::KeyType => {
                            if key_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("keyType"));
                            }
                            key_type__ = Some(map.next_value()?);
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = Some(map.next_value()?);
                        }
                        GeneratedField::ValueType => {
                            if value_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("valueType"));
                            }
                            value_type__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(WriteTableData {
                    key: key__.unwrap_or_default(),
                    key_type: key_type__.unwrap_or_default(),
                    value: value__.unwrap_or_default(),
                    value_type: value_type__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WriteTableData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for WriteTableItem {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.state_key_hash.is_empty() {
            len += 1;
        }
        if !self.handle.is_empty() {
            len += 1;
        }
        if !self.key.is_empty() {
            len += 1;
        }
        if self.data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.transaction.v1.WriteTableItem", len)?;
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field("stateKeyHash", pbjson::private::base64::encode(&self.state_key_hash).as_str())?;
        }
        if !self.handle.is_empty() {
            struct_ser.serialize_field("handle", &self.handle)?;
        }
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteTableItem {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "state_key_hash",
            "stateKeyHash",
            "handle",
            "key",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StateKeyHash,
            Handle,
            Key,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "stateKeyHash" | "state_key_hash" => Ok(GeneratedField::StateKeyHash),
                            "handle" => Ok(GeneratedField::Handle),
                            "key" => Ok(GeneratedField::Key),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteTableItem;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.transaction.v1.WriteTableItem")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteTableItem, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut state_key_hash__ = None;
                let mut handle__ = None;
                let mut key__ = None;
                let mut data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::StateKeyHash => {
                            if state_key_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateKeyHash"));
                            }
                            state_key_hash__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Handle => {
                            if handle__.is_some() {
                                return Err(serde::de::Error::duplicate_field("handle"));
                            }
                            handle__ = Some(map.next_value()?);
                        }
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map.next_value()?);
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map.next_value()?;
                        }
                    }
                }
                Ok(WriteTableItem {
                    state_key_hash: state_key_hash__.unwrap_or_default(),
                    handle: handle__.unwrap_or_default(),
                    key: key__.unwrap_or_default(),
                    data: data__,
                })
            }
        }
        deserializer.deserialize_struct("velor.transaction.v1.WriteTableItem", FIELDS, GeneratedVisitor)
    }
}
