// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// @generated
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.AccountSignature", len)?;
        if self.r#type != 0 {
            let v = account_signature::Type::from_i32(self.r#type).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.r#type))
            })?;
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
        const FIELDS: &[&str] = &["type", "ed25519", "multiEd25519"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Ed25519,
            MultiEd25519,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "multiEd25519" => Ok(GeneratedField::MultiEd25519),
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
                formatter.write_str("struct aptos.extractor.v1.AccountSignature")
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
                            signature__ =
                                Some(account_signature::Signature::Ed25519(map.next_value()?));
                        }
                        GeneratedField::MultiEd25519 => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiEd25519"));
                            }
                            signature__ = Some(account_signature::Signature::MultiEd25519(
                                map.next_value()?,
                            ));
                        }
                    }
                }
                Ok(AccountSignature {
                    r#type: r#type__.unwrap_or_default(),
                    signature: signature__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.AccountSignature",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for account_signature::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Ed25519 => "ED25519",
            Self::MultiEd25519 => "MULTI_ED25519",
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
        const FIELDS: &[&str] = &["ED25519", "MULTI_ED25519"];

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
                    "ED25519" => Ok(account_signature::Type::Ed25519),
                    "MULTI_ED25519" => Ok(account_signature::Type::MultiEd25519),
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.Block", len)?;
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
        const FIELDS: &[&str] = &["timestamp", "height", "transactions", "chainId"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "chainId" => Ok(GeneratedField::ChainId),
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
                formatter.write_str("struct aptos.extractor.v1.Block")
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
                            timestamp__ = Some(map.next_value()?);
                        }
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
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
                            chain_id__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
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
        deserializer.deserialize_struct("aptos.extractor.v1.Block", FIELDS, GeneratedVisitor)
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.BlockMetadataTransaction", len)?;
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
            struct_ser.serialize_field(
                "previousBlockVotesBitvec",
                pbjson::private::base64::encode(&self.previous_block_votes_bitvec).as_str(),
            )?;
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
            "previousBlockVotesBitvec",
            "proposer",
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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "previousBlockVotesBitvec" => {
                                Ok(GeneratedField::PreviousBlockVotesBitvec)
                            }
                            "proposer" => Ok(GeneratedField::Proposer),
                            "failedProposerIndices" => Ok(GeneratedField::FailedProposerIndices),
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
                formatter.write_str("struct aptos.extractor.v1.BlockMetadataTransaction")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<BlockMetadataTransaction, V::Error>
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
                            round__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                        GeneratedField::PreviousBlockVotesBitvec => {
                            if previous_block_votes_bitvec__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "previousBlockVotesBitvec",
                                ));
                            }
                            previous_block_votes_bitvec__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Proposer => {
                            if proposer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proposer"));
                            }
                            proposer__ = Some(map.next_value()?);
                        }
                        GeneratedField::FailedProposerIndices => {
                            if failed_proposer_indices__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "failedProposerIndices",
                                ));
                            }
                            failed_proposer_indices__ = Some(
                                map.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter()
                                    .map(|x| x.0)
                                    .collect(),
                            );
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.BlockMetadataTransaction",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.DeleteModule", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field(
                "stateKeyHash",
                pbjson::private::base64::encode(&self.state_key_hash).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["address", "stateKeyHash", "module"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "stateKeyHash" => Ok(GeneratedField::StateKeyHash),
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
                formatter.write_str("struct aptos.extractor.v1.DeleteModule")
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
                            state_key_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Module => {
                            if module__.is_some() {
                                return Err(serde::de::Error::duplicate_field("module"));
                            }
                            module__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct("aptos.extractor.v1.DeleteModule", FIELDS, GeneratedVisitor)
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.DeleteResource", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field(
                "stateKeyHash",
                pbjson::private::base64::encode(&self.state_key_hash).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["address", "stateKeyHash", "type", "typeStr"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "stateKeyHash" => Ok(GeneratedField::StateKeyHash),
                            "type" => Ok(GeneratedField::Type),
                            "typeStr" => Ok(GeneratedField::TypeStr),
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
                formatter.write_str("struct aptos.extractor.v1.DeleteResource")
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
                            state_key_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.DeleteResource",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.DeleteTableData", len)?;
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
        const FIELDS: &[&str] = &["key", "keyType"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "keyType" => Ok(GeneratedField::KeyType),
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
                formatter.write_str("struct aptos.extractor.v1.DeleteTableData")
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.DeleteTableData",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.DeleteTableItem", len)?;
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field(
                "stateKeyHash",
                pbjson::private::base64::encode(&self.state_key_hash).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["stateKeyHash", "handle", "key", "data"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "stateKeyHash" => Ok(GeneratedField::StateKeyHash),
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
                formatter.write_str("struct aptos.extractor.v1.DeleteTableItem")
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
                            state_key_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
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
                            data__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.DeleteTableItem",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.DirectWriteSet", len)?;
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
        const FIELDS: &[&str] = &["writeSetChange", "events"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "writeSetChange" => Ok(GeneratedField::WriteSetChange),
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
                formatter.write_str("struct aptos.extractor.v1.DirectWriteSet")
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.DirectWriteSet",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.Ed25519Signature", len)?;
        if !self.public_key.is_empty() {
            struct_ser.serialize_field(
                "publicKey",
                pbjson::private::base64::encode(&self.public_key).as_str(),
            )?;
        }
        if !self.signature.is_empty() {
            struct_ser.serialize_field(
                "signature",
                pbjson::private::base64::encode(&self.signature).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["publicKey", "signature"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "publicKey" => Ok(GeneratedField::PublicKey),
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
                formatter.write_str("struct aptos.extractor.v1.Ed25519Signature")
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
                            public_key__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                    }
                }
                Ok(Ed25519Signature {
                    public_key: public_key__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.Ed25519Signature",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.EntryFunctionId", len)?;
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
        const FIELDS: &[&str] = &["module", "name"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.EntryFunctionId")
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
                            module__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.EntryFunctionId",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.EntryFunctionPayload", len)?;
        if let Some(v) = self.function.as_ref() {
            struct_ser.serialize_field("function", v)?;
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
impl<'de> serde::Deserialize<'de> for EntryFunctionPayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["function", "typeArguments", "arguments"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Function,
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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "function" => Ok(GeneratedField::Function),
                            "typeArguments" => Ok(GeneratedField::TypeArguments),
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
            type Value = EntryFunctionPayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.extractor.v1.EntryFunctionPayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EntryFunctionPayload, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut function__ = None;
                let mut type_arguments__ = None;
                let mut arguments__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Function => {
                            if function__.is_some() {
                                return Err(serde::de::Error::duplicate_field("function"));
                            }
                            function__ = Some(map.next_value()?);
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
                Ok(EntryFunctionPayload {
                    function: function__,
                    type_arguments: type_arguments__.unwrap_or_default(),
                    arguments: arguments__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.EntryFunctionPayload",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.Event", len)?;
        if let Some(v) = self.key.as_ref() {
            struct_ser.serialize_field("key", v)?;
        }
        if self.sequence_number != 0 {
            struct_ser.serialize_field(
                "sequenceNumber",
                ToString::to_string(&self.sequence_number).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["key", "sequenceNumber", "type", "typeStr", "data"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "sequenceNumber" => Ok(GeneratedField::SequenceNumber),
                            "type" => Ok(GeneratedField::Type),
                            "typeStr" => Ok(GeneratedField::TypeStr),
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
                formatter.write_str("struct aptos.extractor.v1.Event")
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
                            key__ = Some(map.next_value()?);
                        }
                        GeneratedField::SequenceNumber => {
                            if sequence_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequenceNumber"));
                            }
                            sequence_number__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct("aptos.extractor.v1.Event", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.EventKey", len)?;
        if self.creation_number != 0 {
            struct_ser.serialize_field(
                "creationNumber",
                ToString::to_string(&self.creation_number).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["creationNumber", "accountAddress"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "creationNumber" => Ok(GeneratedField::CreationNumber),
                            "accountAddress" => Ok(GeneratedField::AccountAddress),
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
                formatter.write_str("struct aptos.extractor.v1.EventKey")
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
                            creation_number__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
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
        deserializer.deserialize_struct("aptos.extractor.v1.EventKey", FIELDS, GeneratedVisitor)
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.GenesisTransaction", len)?;
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
        const FIELDS: &[&str] = &["payload", "events"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.GenesisTransaction")
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
                            payload__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.GenesisTransaction",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for ModuleBundlePayload {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.modules.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.ModuleBundlePayload", len)?;
        if !self.modules.is_empty() {
            struct_ser.serialize_field("modules", &self.modules)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ModuleBundlePayload {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["modules"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Modules,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "modules" => Ok(GeneratedField::Modules),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ModuleBundlePayload;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.extractor.v1.ModuleBundlePayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ModuleBundlePayload, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut modules__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Modules => {
                            if modules__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modules"));
                            }
                            modules__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ModuleBundlePayload {
                    modules: modules__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.ModuleBundlePayload",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for MoveAbility {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Copy => "COPY",
            Self::Drop => "DROP",
            Self::Store => "STORE",
            Self::Key => "KEY",
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
        const FIELDS: &[&str] = &["COPY", "DROP", "STORE", "KEY"];

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
                    "COPY" => Ok(MoveAbility::Copy),
                    "DROP" => Ok(MoveAbility::Drop),
                    "STORE" => Ok(MoveAbility::Store),
                    "KEY" => Ok(MoveAbility::Key),
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.MoveFunction", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if self.visibility != 0 {
            let v = move_function::Visibility::from_i32(self.visibility).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.visibility))
            })?;
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
            "isEntry",
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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "isEntry" => Ok(GeneratedField::IsEntry),
                            "genericTypeParams" => Ok(GeneratedField::GenericTypeParams),
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
                formatter.write_str("struct aptos.extractor.v1.MoveFunction")
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
                            visibility__ =
                                Some(map.next_value::<move_function::Visibility>()? as i32);
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
        deserializer.deserialize_struct("aptos.extractor.v1.MoveFunction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for move_function::Visibility {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Private => "PRIVATE",
            Self::Public => "PUBLIC",
            Self::Friend => "FRIEND",
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
        const FIELDS: &[&str] = &["PRIVATE", "PUBLIC", "FRIEND"];

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
                    "PRIVATE" => Ok(move_function::Visibility::Private),
                    "PUBLIC" => Ok(move_function::Visibility::Public),
                    "FRIEND" => Ok(move_function::Visibility::Friend),
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MoveFunctionGenericTypeParam", len)?;
        if !self.constraints.is_empty() {
            let v = self
                .constraints
                .iter()
                .cloned()
                .map(|v| {
                    MoveAbility::from_i32(v)
                        .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                })
                .collect::<Result<Vec<_>, _>>()?;
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
        const FIELDS: &[&str] = &["constraints"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.MoveFunctionGenericTypeParam")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<MoveFunctionGenericTypeParam, V::Error>
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
                            constraints__ = Some(
                                map.next_value::<Vec<MoveAbility>>()?
                                    .into_iter()
                                    .map(|x| x as i32)
                                    .collect(),
                            );
                        }
                    }
                }
                Ok(MoveFunctionGenericTypeParam {
                    constraints: constraints__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MoveFunctionGenericTypeParam",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.MoveModule", len)?;
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
        const FIELDS: &[&str] = &["address", "name", "friends", "exposedFunctions", "structs"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "exposedFunctions" => Ok(GeneratedField::ExposedFunctions),
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
                formatter.write_str("struct aptos.extractor.v1.MoveModule")
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
        deserializer.deserialize_struct("aptos.extractor.v1.MoveModule", FIELDS, GeneratedVisitor)
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MoveModuleBytecode", len)?;
        if !self.bytecode.is_empty() {
            struct_ser.serialize_field(
                "bytecode",
                pbjson::private::base64::encode(&self.bytecode).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["bytecode", "abi"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.MoveModuleBytecode")
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
                            bytecode__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Abi => {
                            if abi__.is_some() {
                                return Err(serde::de::Error::duplicate_field("abi"));
                            }
                            abi__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveModuleBytecode {
                    bytecode: bytecode__.unwrap_or_default(),
                    abi: abi__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MoveModuleBytecode",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.MoveModuleId", len)?;
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
        const FIELDS: &[&str] = &["address", "name"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.MoveModuleId")
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
        deserializer.deserialize_struct("aptos.extractor.v1.MoveModuleId", FIELDS, GeneratedVisitor)
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MoveScriptBytecode", len)?;
        if !self.bytecode.is_empty() {
            struct_ser.serialize_field(
                "bytecode",
                pbjson::private::base64::encode(&self.bytecode).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["bytecode", "abi"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.MoveScriptBytecode")
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
                            bytecode__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Abi => {
                            if abi__.is_some() {
                                return Err(serde::de::Error::duplicate_field("abi"));
                            }
                            abi__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveScriptBytecode {
                    bytecode: bytecode__.unwrap_or_default(),
                    abi: abi__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MoveScriptBytecode",
            FIELDS,
            GeneratedVisitor,
        )
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
        if !self.abilities.is_empty() {
            len += 1;
        }
        if !self.generic_type_params.is_empty() {
            len += 1;
        }
        if !self.fields.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.MoveStruct", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if self.is_native {
            struct_ser.serialize_field("isNative", &self.is_native)?;
        }
        if !self.abilities.is_empty() {
            let v = self
                .abilities
                .iter()
                .cloned()
                .map(|v| {
                    MoveAbility::from_i32(v)
                        .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                })
                .collect::<Result<Vec<_>, _>>()?;
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
            "isNative",
            "abilities",
            "genericTypeParams",
            "fields",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            IsNative,
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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "isNative" => Ok(GeneratedField::IsNative),
                            "abilities" => Ok(GeneratedField::Abilities),
                            "genericTypeParams" => Ok(GeneratedField::GenericTypeParams),
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
                formatter.write_str("struct aptos.extractor.v1.MoveStruct")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveStruct, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut is_native__ = None;
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
                        GeneratedField::Abilities => {
                            if abilities__.is_some() {
                                return Err(serde::de::Error::duplicate_field("abilities"));
                            }
                            abilities__ = Some(
                                map.next_value::<Vec<MoveAbility>>()?
                                    .into_iter()
                                    .map(|x| x as i32)
                                    .collect(),
                            );
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
                    abilities: abilities__.unwrap_or_default(),
                    generic_type_params: generic_type_params__.unwrap_or_default(),
                    fields: fields__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.extractor.v1.MoveStruct", FIELDS, GeneratedVisitor)
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MoveStructField", len)?;
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
        const FIELDS: &[&str] = &["name", "type"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.MoveStructField")
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
                            r#type__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MoveStructField {
                    name: name__.unwrap_or_default(),
                    r#type: r#type__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MoveStructField",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MoveStructGenericTypeParam", len)?;
        if !self.constraints.is_empty() {
            let v = self
                .constraints
                .iter()
                .cloned()
                .map(|v| {
                    MoveAbility::from_i32(v)
                        .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                })
                .collect::<Result<Vec<_>, _>>()?;
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
        const FIELDS: &[&str] = &["constraints", "isPhantom"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "constraints" => Ok(GeneratedField::Constraints),
                            "isPhantom" => Ok(GeneratedField::IsPhantom),
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
                formatter.write_str("struct aptos.extractor.v1.MoveStructGenericTypeParam")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<MoveStructGenericTypeParam, V::Error>
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
                            constraints__ = Some(
                                map.next_value::<Vec<MoveAbility>>()?
                                    .into_iter()
                                    .map(|x| x as i32)
                                    .collect(),
                            );
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MoveStructGenericTypeParam",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MoveStructTag", len)?;
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
        const FIELDS: &[&str] = &["address", "module", "name", "genericTypeParams"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "genericTypeParams" => Ok(GeneratedField::GenericTypeParams),
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
                formatter.write_str("struct aptos.extractor.v1.MoveStructTag")
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MoveStructTag",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.MoveType", len)?;
        if self.r#type != 0 {
            let v = MoveTypes::from_i32(self.r#type).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.r#type))
            })?;
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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "genericTypeParamIndex" => Ok(GeneratedField::GenericTypeParamIndex),
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
                formatter.write_str("struct aptos.extractor.v1.MoveType")
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
                            content__ = Some(move_type::Content::Vector(map.next_value()?));
                        }
                        GeneratedField::Struct => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("struct"));
                            }
                            content__ = Some(move_type::Content::Struct(map.next_value()?));
                        }
                        GeneratedField::GenericTypeParamIndex => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "genericTypeParamIndex",
                                ));
                            }
                            content__ = Some(move_type::Content::GenericTypeParamIndex(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            ));
                        }
                        GeneratedField::Reference => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reference"));
                            }
                            content__ = Some(move_type::Content::Reference(map.next_value()?));
                        }
                        GeneratedField::Unparsable => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("unparsable"));
                            }
                            content__ = Some(move_type::Content::Unparsable(map.next_value()?));
                        }
                    }
                }
                Ok(MoveType {
                    r#type: r#type__.unwrap_or_default(),
                    content: content__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.extractor.v1.MoveType", FIELDS, GeneratedVisitor)
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MoveType.ReferenceType", len)?;
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
        const FIELDS: &[&str] = &["mutable", "to"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.MoveType.ReferenceType")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<move_type::ReferenceType, V::Error>
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
                            to__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(move_type::ReferenceType {
                    mutable: mutable__.unwrap_or_default(),
                    to: to__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MoveType.ReferenceType",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for MoveTypes {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Bool => "Bool",
            Self::U8 => "U8",
            Self::U64 => "U64",
            Self::U128 => "U128",
            Self::Address => "Address",
            Self::Signer => "Signer",
            Self::Vector => "Vector",
            Self::Struct => "Struct",
            Self::GenericTypeParam => "GenericTypeParam",
            Self::Reference => "Reference",
            Self::Unparsable => "Unparsable",
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
            "Bool",
            "U8",
            "U64",
            "U128",
            "Address",
            "Signer",
            "Vector",
            "Struct",
            "GenericTypeParam",
            "Reference",
            "Unparsable",
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
                    "Bool" => Ok(MoveTypes::Bool),
                    "U8" => Ok(MoveTypes::U8),
                    "U64" => Ok(MoveTypes::U64),
                    "U128" => Ok(MoveTypes::U128),
                    "Address" => Ok(MoveTypes::Address),
                    "Signer" => Ok(MoveTypes::Signer),
                    "Vector" => Ok(MoveTypes::Vector),
                    "Struct" => Ok(MoveTypes::Struct),
                    "GenericTypeParam" => Ok(MoveTypes::GenericTypeParam),
                    "Reference" => Ok(MoveTypes::Reference),
                    "Unparsable" => Ok(MoveTypes::Unparsable),
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MultiAgentSignature", len)?;
        if let Some(v) = self.sender.as_ref() {
            struct_ser.serialize_field("sender", v)?;
        }
        if !self.secondary_signer_addresses.is_empty() {
            struct_ser
                .serialize_field("secondarySignerAddresses", &self.secondary_signer_addresses)?;
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
        const FIELDS: &[&str] = &["sender", "secondarySignerAddresses", "secondarySigners"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "sender" => Ok(GeneratedField::Sender),
                            "secondarySignerAddresses" => {
                                Ok(GeneratedField::SecondarySignerAddresses)
                            }
                            "secondarySigners" => Ok(GeneratedField::SecondarySigners),
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
                formatter.write_str("struct aptos.extractor.v1.MultiAgentSignature")
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
                            sender__ = Some(map.next_value()?);
                        }
                        GeneratedField::SecondarySignerAddresses => {
                            if secondary_signer_addresses__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "secondarySignerAddresses",
                                ));
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MultiAgentSignature",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.MultiEd25519Signature", len)?;
        if !self.public_keys.is_empty() {
            struct_ser.serialize_field(
                "publicKeys",
                &self
                    .public_keys
                    .iter()
                    .map(pbjson::private::base64::encode)
                    .collect::<Vec<_>>(),
            )?;
        }
        if !self.signatures.is_empty() {
            struct_ser.serialize_field(
                "signatures",
                &self
                    .signatures
                    .iter()
                    .map(pbjson::private::base64::encode)
                    .collect::<Vec<_>>(),
            )?;
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
        const FIELDS: &[&str] = &["publicKeys", "signatures", "threshold", "publicKeyIndices"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "publicKeys" => Ok(GeneratedField::PublicKeys),
                            "signatures" => Ok(GeneratedField::Signatures),
                            "threshold" => Ok(GeneratedField::Threshold),
                            "publicKeyIndices" => Ok(GeneratedField::PublicKeyIndices),
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
                formatter.write_str("struct aptos.extractor.v1.MultiEd25519Signature")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<MultiEd25519Signature, V::Error>
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
                            public_keys__ = Some(
                                map.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter()
                                    .map(|x| x.0)
                                    .collect(),
                            );
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = Some(
                                map.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter()
                                    .map(|x| x.0)
                                    .collect(),
                            );
                        }
                        GeneratedField::Threshold => {
                            if threshold__.is_some() {
                                return Err(serde::de::Error::duplicate_field("threshold"));
                            }
                            threshold__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::PublicKeyIndices => {
                            if public_key_indices__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKeyIndices"));
                            }
                            public_key_indices__ = Some(
                                map.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter()
                                    .map(|x| x.0)
                                    .collect(),
                            );
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.MultiEd25519Signature",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.ScriptPayload", len)?;
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
        const FIELDS: &[&str] = &["code", "typeArguments", "arguments"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "code" => Ok(GeneratedField::Code),
                            "typeArguments" => Ok(GeneratedField::TypeArguments),
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
                formatter.write_str("struct aptos.extractor.v1.ScriptPayload")
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
                            code__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.ScriptPayload",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.ScriptWriteSet", len)?;
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
        const FIELDS: &[&str] = &["executeAs", "script"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "executeAs" => Ok(GeneratedField::ExecuteAs),
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
                formatter.write_str("struct aptos.extractor.v1.ScriptWriteSet")
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
                            script__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ScriptWriteSet {
                    execute_as: execute_as__.unwrap_or_default(),
                    script: script__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.ScriptWriteSet",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.Signature", len)?;
        if self.r#type != 0 {
            let v = signature::Type::from_i32(self.r#type).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.r#type))
            })?;
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
        const FIELDS: &[&str] = &["type", "ed25519", "multiEd25519", "multiAgent"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Ed25519,
            MultiEd25519,
            MultiAgent,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "multiEd25519" => Ok(GeneratedField::MultiEd25519),
                            "multiAgent" => Ok(GeneratedField::MultiAgent),
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
                formatter.write_str("struct aptos.extractor.v1.Signature")
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
                            signature__ = Some(signature::Signature::Ed25519(map.next_value()?));
                        }
                        GeneratedField::MultiEd25519 => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiEd25519"));
                            }
                            signature__ =
                                Some(signature::Signature::MultiEd25519(map.next_value()?));
                        }
                        GeneratedField::MultiAgent => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiAgent"));
                            }
                            signature__ = Some(signature::Signature::MultiAgent(map.next_value()?));
                        }
                    }
                }
                Ok(Signature {
                    r#type: r#type__.unwrap_or_default(),
                    signature: signature__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.extractor.v1.Signature", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for signature::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Ed25519 => "ED25519",
            Self::MultiEd25519 => "MULTI_ED25519",
            Self::MultiAgent => "MULTI_AGENT",
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
        const FIELDS: &[&str] = &["ED25519", "MULTI_ED25519", "MULTI_AGENT"];

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
                    "ED25519" => Ok(signature::Type::Ed25519),
                    "MULTI_ED25519" => Ok(signature::Type::MultiEd25519),
                    "MULTI_AGENT" => Ok(signature::Type::MultiAgent),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
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
        let struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.StateCheckpointTransaction", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StateCheckpointTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {}
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.StateCheckpointTransaction")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<StateCheckpointTransaction, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(StateCheckpointTransaction {})
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.StateCheckpointTransaction",
            FIELDS,
            GeneratedVisitor,
        )
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
        if self.txn_data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.Transaction", len)?;
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
            struct_ser.serialize_field(
                "blockHeight",
                ToString::to_string(&self.block_height).as_str(),
            )?;
        }
        if self.r#type != 0 {
            let v = transaction::TransactionType::from_i32(self.r#type).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.r#type))
            })?;
            struct_ser.serialize_field("type", &v)?;
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
            "blockHeight",
            "type",
            "blockMetadata",
            "genesis",
            "stateCheckpoint",
            "user",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Timestamp,
            Version,
            Info,
            Epoch,
            BlockHeight,
            Type,
            BlockMetadata,
            Genesis,
            StateCheckpoint,
            User,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            "blockHeight" => Ok(GeneratedField::BlockHeight),
                            "type" => Ok(GeneratedField::Type),
                            "blockMetadata" => Ok(GeneratedField::BlockMetadata),
                            "genesis" => Ok(GeneratedField::Genesis),
                            "stateCheckpoint" => Ok(GeneratedField::StateCheckpoint),
                            "user" => Ok(GeneratedField::User),
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
                formatter.write_str("struct aptos.extractor.v1.Transaction")
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
                let mut txn_data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = Some(map.next_value()?);
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Info => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("info"));
                            }
                            info__ = Some(map.next_value()?);
                        }
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ =
                                Some(map.next_value::<transaction::TransactionType>()? as i32);
                        }
                        GeneratedField::BlockMetadata => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockMetadata"));
                            }
                            txn_data__ =
                                Some(transaction::TxnData::BlockMetadata(map.next_value()?));
                        }
                        GeneratedField::Genesis => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genesis"));
                            }
                            txn_data__ = Some(transaction::TxnData::Genesis(map.next_value()?));
                        }
                        GeneratedField::StateCheckpoint => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateCheckpoint"));
                            }
                            txn_data__ =
                                Some(transaction::TxnData::StateCheckpoint(map.next_value()?));
                        }
                        GeneratedField::User => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("user"));
                            }
                            txn_data__ = Some(transaction::TxnData::User(map.next_value()?));
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
                    txn_data: txn_data__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.extractor.v1.Transaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction::TransactionType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Genesis => "GENESIS",
            Self::BlockMetadata => "BLOCK_METADATA",
            Self::StateCheckpoint => "STATE_CHECKPOINT",
            Self::User => "USER",
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
        const FIELDS: &[&str] = &["GENESIS", "BLOCK_METADATA", "STATE_CHECKPOINT", "USER"];

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
                    "GENESIS" => Ok(transaction::TransactionType::Genesis),
                    "BLOCK_METADATA" => Ok(transaction::TransactionType::BlockMetadata),
                    "STATE_CHECKPOINT" => Ok(transaction::TransactionType::StateCheckpoint),
                    "USER" => Ok(transaction::TransactionType::User),
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.TransactionInfo", len)?;
        if !self.hash.is_empty() {
            struct_ser
                .serialize_field("hash", pbjson::private::base64::encode(&self.hash).as_str())?;
        }
        if !self.state_change_hash.is_empty() {
            struct_ser.serialize_field(
                "stateChangeHash",
                pbjson::private::base64::encode(&self.state_change_hash).as_str(),
            )?;
        }
        if !self.event_root_hash.is_empty() {
            struct_ser.serialize_field(
                "eventRootHash",
                pbjson::private::base64::encode(&self.event_root_hash).as_str(),
            )?;
        }
        if let Some(v) = self.state_checkpoint_hash.as_ref() {
            struct_ser.serialize_field(
                "stateCheckpointHash",
                pbjson::private::base64::encode(&v).as_str(),
            )?;
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
            struct_ser.serialize_field(
                "accumulatorRootHash",
                pbjson::private::base64::encode(&self.accumulator_root_hash).as_str(),
            )?;
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
            "stateChangeHash",
            "eventRootHash",
            "stateCheckpointHash",
            "gasUsed",
            "success",
            "vmStatus",
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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "hash" => Ok(GeneratedField::Hash),
                            "stateChangeHash" => Ok(GeneratedField::StateChangeHash),
                            "eventRootHash" => Ok(GeneratedField::EventRootHash),
                            "stateCheckpointHash" => Ok(GeneratedField::StateCheckpointHash),
                            "gasUsed" => Ok(GeneratedField::GasUsed),
                            "success" => Ok(GeneratedField::Success),
                            "vmStatus" => Ok(GeneratedField::VmStatus),
                            "accumulatorRootHash" => Ok(GeneratedField::AccumulatorRootHash),
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
                formatter.write_str("struct aptos.extractor.v1.TransactionInfo")
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
                            hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::StateChangeHash => {
                            if state_change_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateChangeHash"));
                            }
                            state_change_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::EventRootHash => {
                            if event_root_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("eventRootHash"));
                            }
                            event_root_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::StateCheckpointHash => {
                            if state_checkpoint_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "stateCheckpointHash",
                                ));
                            }
                            state_checkpoint_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::GasUsed => {
                            if gas_used__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gasUsed"));
                            }
                            gas_used__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
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
                                return Err(serde::de::Error::duplicate_field(
                                    "accumulatorRootHash",
                                ));
                            }
                            accumulator_root_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.TransactionInfo",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.TransactionPayload", len)?;
        if self.r#type != 0 {
            let v = transaction_payload::Type::from_i32(self.r#type).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.r#type))
            })?;
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
                transaction_payload::Payload::ModuleBundlePayload(v) => {
                    struct_ser.serialize_field("moduleBundlePayload", v)?;
                }
                transaction_payload::Payload::WriteSetPayload(v) => {
                    struct_ser.serialize_field("writeSetPayload", v)?;
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
            "entryFunctionPayload",
            "scriptPayload",
            "moduleBundlePayload",
            "writeSetPayload",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            EntryFunctionPayload,
            ScriptPayload,
            ModuleBundlePayload,
            WriteSetPayload,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "entryFunctionPayload" => Ok(GeneratedField::EntryFunctionPayload),
                            "scriptPayload" => Ok(GeneratedField::ScriptPayload),
                            "moduleBundlePayload" => Ok(GeneratedField::ModuleBundlePayload),
                            "writeSetPayload" => Ok(GeneratedField::WriteSetPayload),
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
                formatter.write_str("struct aptos.extractor.v1.TransactionPayload")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionPayload, V::Error>
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
                            r#type__ = Some(map.next_value::<transaction_payload::Type>()? as i32);
                        }
                        GeneratedField::EntryFunctionPayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "entryFunctionPayload",
                                ));
                            }
                            payload__ = Some(transaction_payload::Payload::EntryFunctionPayload(
                                map.next_value()?,
                            ));
                        }
                        GeneratedField::ScriptPayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("scriptPayload"));
                            }
                            payload__ = Some(transaction_payload::Payload::ScriptPayload(
                                map.next_value()?,
                            ));
                        }
                        GeneratedField::ModuleBundlePayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "moduleBundlePayload",
                                ));
                            }
                            payload__ = Some(transaction_payload::Payload::ModuleBundlePayload(
                                map.next_value()?,
                            ));
                        }
                        GeneratedField::WriteSetPayload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeSetPayload"));
                            }
                            payload__ = Some(transaction_payload::Payload::WriteSetPayload(
                                map.next_value()?,
                            ));
                        }
                    }
                }
                Ok(TransactionPayload {
                    r#type: r#type__.unwrap_or_default(),
                    payload: payload__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.TransactionPayload",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for transaction_payload::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::EntryFunctionPayload => "ENTRY_FUNCTION_PAYLOAD",
            Self::ScriptPayload => "SCRIPT_PAYLOAD",
            Self::ModuleBundlePayload => "MODULE_BUNDLE_PAYLOAD",
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
            "ENTRY_FUNCTION_PAYLOAD",
            "SCRIPT_PAYLOAD",
            "MODULE_BUNDLE_PAYLOAD",
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
                    "ENTRY_FUNCTION_PAYLOAD" => Ok(transaction_payload::Type::EntryFunctionPayload),
                    "SCRIPT_PAYLOAD" => Ok(transaction_payload::Type::ScriptPayload),
                    "MODULE_BUNDLE_PAYLOAD" => Ok(transaction_payload::Type::ModuleBundlePayload),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionTrimmed {
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.TransactionTrimmed", len)?;
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionTrimmed {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["timestamp", "version"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Timestamp,
            Version,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionTrimmed;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.extractor.v1.TransactionTrimmed")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionTrimmed, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut timestamp__ = None;
                let mut version__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = Some(map.next_value()?);
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                    }
                }
                Ok(TransactionTrimmed {
                    timestamp: timestamp__,
                    version: version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.TransactionTrimmed",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.UserTransaction", len)?;
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
        const FIELDS: &[&str] = &["request", "events"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                formatter.write_str("struct aptos.extractor.v1.UserTransaction")
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
                            request__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.UserTransaction",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.UserTransactionRequest", len)?;
        if !self.sender.is_empty() {
            struct_ser.serialize_field("sender", &self.sender)?;
        }
        if self.sequence_number != 0 {
            struct_ser.serialize_field(
                "sequenceNumber",
                ToString::to_string(&self.sequence_number).as_str(),
            )?;
        }
        if self.max_gas_amount != 0 {
            struct_ser.serialize_field(
                "maxGasAmount",
                ToString::to_string(&self.max_gas_amount).as_str(),
            )?;
        }
        if self.gas_unit_price != 0 {
            struct_ser.serialize_field(
                "gasUnitPrice",
                ToString::to_string(&self.gas_unit_price).as_str(),
            )?;
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
            "sequenceNumber",
            "maxGasAmount",
            "gasUnitPrice",
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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "sender" => Ok(GeneratedField::Sender),
                            "sequenceNumber" => Ok(GeneratedField::SequenceNumber),
                            "maxGasAmount" => Ok(GeneratedField::MaxGasAmount),
                            "gasUnitPrice" => Ok(GeneratedField::GasUnitPrice),
                            "expirationTimestampSecs" => {
                                Ok(GeneratedField::ExpirationTimestampSecs)
                            }
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
                formatter.write_str("struct aptos.extractor.v1.UserTransactionRequest")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<UserTransactionRequest, V::Error>
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
                            sequence_number__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::MaxGasAmount => {
                            if max_gas_amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxGasAmount"));
                            }
                            max_gas_amount__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::GasUnitPrice => {
                            if gas_unit_price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gasUnitPrice"));
                            }
                            gas_unit_price__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::ExpirationTimestampSecs => {
                            if expiration_timestamp_secs__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "expirationTimestampSecs",
                                ));
                            }
                            expiration_timestamp_secs__ = Some(map.next_value()?);
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = Some(map.next_value()?);
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.UserTransactionRequest",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.WriteModule", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field(
                "stateKeyHash",
                pbjson::private::base64::encode(&self.state_key_hash).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["address", "stateKeyHash", "data"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "stateKeyHash" => Ok(GeneratedField::StateKeyHash),
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
                formatter.write_str("struct aptos.extractor.v1.WriteModule")
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
                            state_key_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct("aptos.extractor.v1.WriteModule", FIELDS, GeneratedVisitor)
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.WriteResource", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field(
                "stateKeyHash",
                pbjson::private::base64::encode(&self.state_key_hash).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["address", "stateKeyHash", "type", "typeStr", "data"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "address" => Ok(GeneratedField::Address),
                            "stateKeyHash" => Ok(GeneratedField::StateKeyHash),
                            "type" => Ok(GeneratedField::Type),
                            "typeStr" => Ok(GeneratedField::TypeStr),
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
                formatter.write_str("struct aptos.extractor.v1.WriteResource")
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
                            state_key_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.WriteResource",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser = serializer.serialize_struct("aptos.extractor.v1.WriteSet", len)?;
        if self.write_set_type != 0 {
            let v = write_set::WriteSetType::from_i32(self.write_set_type).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.write_set_type))
            })?;
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
        const FIELDS: &[&str] = &["writeSetType", "scriptWriteSet", "directWriteSet"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "writeSetType" => Ok(GeneratedField::WriteSetType),
                            "scriptWriteSet" => Ok(GeneratedField::ScriptWriteSet),
                            "directWriteSet" => Ok(GeneratedField::DirectWriteSet),
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
                formatter.write_str("struct aptos.extractor.v1.WriteSet")
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
                            write_set_type__ =
                                Some(map.next_value::<write_set::WriteSetType>()? as i32);
                        }
                        GeneratedField::ScriptWriteSet => {
                            if write_set__.is_some() {
                                return Err(serde::de::Error::duplicate_field("scriptWriteSet"));
                            }
                            write_set__ =
                                Some(write_set::WriteSet::ScriptWriteSet(map.next_value()?));
                        }
                        GeneratedField::DirectWriteSet => {
                            if write_set__.is_some() {
                                return Err(serde::de::Error::duplicate_field("directWriteSet"));
                            }
                            write_set__ =
                                Some(write_set::WriteSet::DirectWriteSet(map.next_value()?));
                        }
                    }
                }
                Ok(WriteSet {
                    write_set_type: write_set_type__.unwrap_or_default(),
                    write_set: write_set__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.extractor.v1.WriteSet", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for write_set::WriteSetType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::ScriptWriteSet => "SCRIPT_WRITE_SET",
            Self::DirectWriteSet => "DIRECT_WRITE_SET",
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
        const FIELDS: &[&str] = &["SCRIPT_WRITE_SET", "DIRECT_WRITE_SET"];

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
                    "SCRIPT_WRITE_SET" => Ok(write_set::WriteSetType::ScriptWriteSet),
                    "DIRECT_WRITE_SET" => Ok(write_set::WriteSetType::DirectWriteSet),
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.WriteSetChange", len)?;
        if self.r#type != 0 {
            let v = write_set_change::Type::from_i32(self.r#type).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.r#type))
            })?;
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
            "deleteModule",
            "deleteResource",
            "deleteTableItem",
            "writeModule",
            "writeResource",
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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "deleteModule" => Ok(GeneratedField::DeleteModule),
                            "deleteResource" => Ok(GeneratedField::DeleteResource),
                            "deleteTableItem" => Ok(GeneratedField::DeleteTableItem),
                            "writeModule" => Ok(GeneratedField::WriteModule),
                            "writeResource" => Ok(GeneratedField::WriteResource),
                            "writeTableItem" => Ok(GeneratedField::WriteTableItem),
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
                formatter.write_str("struct aptos.extractor.v1.WriteSetChange")
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
                            change__ =
                                Some(write_set_change::Change::DeleteModule(map.next_value()?));
                        }
                        GeneratedField::DeleteResource => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deleteResource"));
                            }
                            change__ =
                                Some(write_set_change::Change::DeleteResource(map.next_value()?));
                        }
                        GeneratedField::DeleteTableItem => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deleteTableItem"));
                            }
                            change__ =
                                Some(write_set_change::Change::DeleteTableItem(map.next_value()?));
                        }
                        GeneratedField::WriteModule => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeModule"));
                            }
                            change__ =
                                Some(write_set_change::Change::WriteModule(map.next_value()?));
                        }
                        GeneratedField::WriteResource => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeResource"));
                            }
                            change__ =
                                Some(write_set_change::Change::WriteResource(map.next_value()?));
                        }
                        GeneratedField::WriteTableItem => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeTableItem"));
                            }
                            change__ =
                                Some(write_set_change::Change::WriteTableItem(map.next_value()?));
                        }
                    }
                }
                Ok(WriteSetChange {
                    r#type: r#type__.unwrap_or_default(),
                    change: change__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.WriteSetChange",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for write_set_change::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::DeleteModule => "DELETE_MODULE",
            Self::DeleteResource => "DELETE_RESOURCE",
            Self::DeleteTableItem => "DELETE_TABLE_ITEM",
            Self::WriteModule => "WRITE_MODULE",
            Self::WriteResource => "WRITE_RESOURCE",
            Self::WriteTableItem => "WRITE_TABLE_ITEM",
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
            "DELETE_MODULE",
            "DELETE_RESOURCE",
            "DELETE_TABLE_ITEM",
            "WRITE_MODULE",
            "WRITE_RESOURCE",
            "WRITE_TABLE_ITEM",
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
                    "DELETE_MODULE" => Ok(write_set_change::Type::DeleteModule),
                    "DELETE_RESOURCE" => Ok(write_set_change::Type::DeleteResource),
                    "DELETE_TABLE_ITEM" => Ok(write_set_change::Type::DeleteTableItem),
                    "WRITE_MODULE" => Ok(write_set_change::Type::WriteModule),
                    "WRITE_RESOURCE" => Ok(write_set_change::Type::WriteResource),
                    "WRITE_TABLE_ITEM" => Ok(write_set_change::Type::WriteTableItem),
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.WriteSetPayload", len)?;
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
        const FIELDS: &[&str] = &["writeSet"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "writeSet" => Ok(GeneratedField::WriteSet),
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
                formatter.write_str("struct aptos.extractor.v1.WriteSetPayload")
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
                            write_set__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(WriteSetPayload {
                    write_set: write_set__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.extractor.v1.WriteSetPayload",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.WriteTableData", len)?;
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
        const FIELDS: &[&str] = &["key", "keyType", "value", "valueType"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "keyType" => Ok(GeneratedField::KeyType),
                            "value" => Ok(GeneratedField::Value),
                            "valueType" => Ok(GeneratedField::ValueType),
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
                formatter.write_str("struct aptos.extractor.v1.WriteTableData")
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.WriteTableData",
            FIELDS,
            GeneratedVisitor,
        )
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.extractor.v1.WriteTableItem", len)?;
        if !self.state_key_hash.is_empty() {
            struct_ser.serialize_field(
                "stateKeyHash",
                pbjson::private::base64::encode(&self.state_key_hash).as_str(),
            )?;
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
        const FIELDS: &[&str] = &["stateKeyHash", "handle", "key", "data"];

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

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "stateKeyHash" => Ok(GeneratedField::StateKeyHash),
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
                formatter.write_str("struct aptos.extractor.v1.WriteTableItem")
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
                            state_key_hash__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
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
                            data__ = Some(map.next_value()?);
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
        deserializer.deserialize_struct(
            "aptos.extractor.v1.WriteTableItem",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
