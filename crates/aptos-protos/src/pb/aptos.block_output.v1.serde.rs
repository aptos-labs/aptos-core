// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// @generated
impl serde::Serialize for BlockMetadataTransactionOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.version != 0 {
            len += 1;
        }
        if !self.id.is_empty() {
            len += 1;
        }
        if self.round != 0 {
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
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.epoch != 0 {
            len += 1;
        }
        let mut struct_ser = serializer
            .serialize_struct("aptos.block_output.v1.BlockMetadataTransactionOutput", len)?;
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if self.round != 0 {
            struct_ser.serialize_field("round", ToString::to_string(&self.round).as_str())?;
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
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if self.epoch != 0 {
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockMetadataTransactionOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "version",
            "id",
            "round",
            "previousBlockVotesBitvec",
            "proposer",
            "failedProposerIndices",
            "timestamp",
            "epoch",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Version,
            Id,
            Round,
            PreviousBlockVotesBitvec,
            Proposer,
            FailedProposerIndices,
            Timestamp,
            Epoch,
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
                            "version" => Ok(GeneratedField::Version),
                            "id" => Ok(GeneratedField::Id),
                            "round" => Ok(GeneratedField::Round),
                            "previousBlockVotesBitvec" => {
                                Ok(GeneratedField::PreviousBlockVotesBitvec)
                            }
                            "proposer" => Ok(GeneratedField::Proposer),
                            "failedProposerIndices" => Ok(GeneratedField::FailedProposerIndices),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "epoch" => Ok(GeneratedField::Epoch),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlockMetadataTransactionOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.BlockMetadataTransactionOutput")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<BlockMetadataTransactionOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut version__ = None;
                let mut id__ = None;
                let mut round__ = None;
                let mut previous_block_votes_bitvec__ = None;
                let mut proposer__ = None;
                let mut failed_proposer_indices__ = None;
                let mut timestamp__ = None;
                let mut epoch__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
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
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = Some(map.next_value()?);
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
                    }
                }
                Ok(BlockMetadataTransactionOutput {
                    version: version__.unwrap_or_default(),
                    id: id__.unwrap_or_default(),
                    round: round__.unwrap_or_default(),
                    previous_block_votes_bitvec: previous_block_votes_bitvec__.unwrap_or_default(),
                    proposer: proposer__.unwrap_or_default(),
                    failed_proposer_indices: failed_proposer_indices__.unwrap_or_default(),
                    timestamp: timestamp__,
                    epoch: epoch__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.BlockMetadataTransactionOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for BlockOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height != 0 {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.chain_id != 0 {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.BlockOutput", len)?;
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
impl<'de> serde::Deserialize<'de> for BlockOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["height", "transactions", "chainId"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
            type Value = BlockOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.BlockOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<BlockOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut transactions__ = None;
                let mut chain_id__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
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
                Ok(BlockOutput {
                    height: height__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                    chain_id: chain_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.BlockOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for EventKeyOutput {
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
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.EventKeyOutput", len)?;
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
impl<'de> serde::Deserialize<'de> for EventKeyOutput {
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
            type Value = EventKeyOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.EventKeyOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventKeyOutput, V::Error>
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
                Ok(EventKeyOutput {
                    creation_number: creation_number__.unwrap_or_default(),
                    account_address: account_address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.EventKeyOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for EventOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.version != 0 {
            len += 1;
        }
        if self.key.is_some() {
            len += 1;
        }
        if self.sequence_number != 0 {
            len += 1;
        }
        if !self.r#type.is_empty() {
            len += 1;
        }
        if !self.type_str.is_empty() {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.EventOutput", len)?;
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if let Some(v) = self.key.as_ref() {
            struct_ser.serialize_field("key", v)?;
        }
        if self.sequence_number != 0 {
            struct_ser.serialize_field(
                "sequenceNumber",
                ToString::to_string(&self.sequence_number).as_str(),
            )?;
        }
        if !self.r#type.is_empty() {
            struct_ser.serialize_field("type", &self.r#type)?;
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
impl<'de> serde::Deserialize<'de> for EventOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "version",
            "key",
            "sequenceNumber",
            "type",
            "typeStr",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Version,
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
                            "version" => Ok(GeneratedField::Version),
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
            type Value = EventOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.EventOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut version__ = None;
                let mut key__ = None;
                let mut sequence_number__ = None;
                let mut r#type__ = None;
                let mut type_str__ = None;
                let mut data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
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
                Ok(EventOutput {
                    version: version__.unwrap_or_default(),
                    key: key__,
                    sequence_number: sequence_number__.unwrap_or_default(),
                    r#type: r#type__.unwrap_or_default(),
                    type_str: type_str__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.EventOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for GenesisTransactionOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.payload.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.GenesisTransactionOutput", len)?;
        if !self.payload.is_empty() {
            struct_ser.serialize_field("payload", &self.payload)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenesisTransactionOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["payload"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenesisTransactionOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.GenesisTransactionOutput")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<GenesisTransactionOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut payload__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(GenesisTransactionOutput {
                    payload: payload__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.GenesisTransactionOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for MoveModuleOutput {
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
        if !self.address.is_empty() {
            len += 1;
        }
        if !self.bytecode.is_empty() {
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
        if self.is_deleted {
            len += 1;
        }
        if self.wsc_index != 0 {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.MoveModuleOutput", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.bytecode.is_empty() {
            struct_ser.serialize_field(
                "bytecode",
                pbjson::private::base64::encode(&self.bytecode).as_str(),
            )?;
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
        if self.is_deleted {
            struct_ser.serialize_field("isDeleted", &self.is_deleted)?;
        }
        if self.wsc_index != 0 {
            struct_ser
                .serialize_field("wscIndex", ToString::to_string(&self.wsc_index).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveModuleOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "address",
            "bytecode",
            "friends",
            "exposedFunctions",
            "structs",
            "isDeleted",
            "wscIndex",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Address,
            Bytecode,
            Friends,
            ExposedFunctions,
            Structs,
            IsDeleted,
            WscIndex,
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
                            "address" => Ok(GeneratedField::Address),
                            "bytecode" => Ok(GeneratedField::Bytecode),
                            "friends" => Ok(GeneratedField::Friends),
                            "exposedFunctions" => Ok(GeneratedField::ExposedFunctions),
                            "structs" => Ok(GeneratedField::Structs),
                            "isDeleted" => Ok(GeneratedField::IsDeleted),
                            "wscIndex" => Ok(GeneratedField::WscIndex),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveModuleOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.MoveModuleOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveModuleOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut address__ = None;
                let mut bytecode__ = None;
                let mut friends__ = None;
                let mut exposed_functions__ = None;
                let mut structs__ = None;
                let mut is_deleted__ = None;
                let mut wsc_index__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = Some(map.next_value()?);
                        }
                        GeneratedField::Bytecode => {
                            if bytecode__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bytecode"));
                            }
                            bytecode__ = Some(
                                map.next_value::<::pbjson::private::BytesDeserialize<_>>()?
                                    .0,
                            );
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
                        GeneratedField::IsDeleted => {
                            if is_deleted__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isDeleted"));
                            }
                            is_deleted__ = Some(map.next_value()?);
                        }
                        GeneratedField::WscIndex => {
                            if wsc_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("wscIndex"));
                            }
                            wsc_index__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                    }
                }
                Ok(MoveModuleOutput {
                    name: name__.unwrap_or_default(),
                    address: address__.unwrap_or_default(),
                    bytecode: bytecode__.unwrap_or_default(),
                    friends: friends__.unwrap_or_default(),
                    exposed_functions: exposed_functions__.unwrap_or_default(),
                    structs: structs__.unwrap_or_default(),
                    is_deleted: is_deleted__.unwrap_or_default(),
                    wsc_index: wsc_index__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.MoveModuleOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for MoveResourceOutput {
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
        if !self.type_str.is_empty() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.generic_type_params.is_empty() {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        if self.is_deleted {
            len += 1;
        }
        if self.wsc_index != 0 {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.MoveResourceOutput", len)?;
        if !self.address.is_empty() {
            struct_ser.serialize_field("address", &self.address)?;
        }
        if !self.module.is_empty() {
            struct_ser.serialize_field("module", &self.module)?;
        }
        if !self.type_str.is_empty() {
            struct_ser.serialize_field("typeStr", &self.type_str)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.generic_type_params.is_empty() {
            struct_ser.serialize_field("genericTypeParams", &self.generic_type_params)?;
        }
        if !self.data.is_empty() {
            struct_ser.serialize_field("data", &self.data)?;
        }
        if self.is_deleted {
            struct_ser.serialize_field("isDeleted", &self.is_deleted)?;
        }
        if self.wsc_index != 0 {
            struct_ser
                .serialize_field("wscIndex", ToString::to_string(&self.wsc_index).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveResourceOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "module",
            "typeStr",
            "name",
            "genericTypeParams",
            "data",
            "isDeleted",
            "wscIndex",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            Module,
            TypeStr,
            Name,
            GenericTypeParams,
            Data,
            IsDeleted,
            WscIndex,
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
                            "typeStr" => Ok(GeneratedField::TypeStr),
                            "name" => Ok(GeneratedField::Name),
                            "genericTypeParams" => Ok(GeneratedField::GenericTypeParams),
                            "data" => Ok(GeneratedField::Data),
                            "isDeleted" => Ok(GeneratedField::IsDeleted),
                            "wscIndex" => Ok(GeneratedField::WscIndex),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveResourceOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.MoveResourceOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveResourceOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut module__ = None;
                let mut type_str__ = None;
                let mut name__ = None;
                let mut generic_type_params__ = None;
                let mut data__ = None;
                let mut is_deleted__ = None;
                let mut wsc_index__ = None;
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
                        GeneratedField::TypeStr => {
                            if type_str__.is_some() {
                                return Err(serde::de::Error::duplicate_field("typeStr"));
                            }
                            type_str__ = Some(map.next_value()?);
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
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = Some(map.next_value()?);
                        }
                        GeneratedField::IsDeleted => {
                            if is_deleted__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isDeleted"));
                            }
                            is_deleted__ = Some(map.next_value()?);
                        }
                        GeneratedField::WscIndex => {
                            if wsc_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("wscIndex"));
                            }
                            wsc_index__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                    }
                }
                Ok(MoveResourceOutput {
                    address: address__.unwrap_or_default(),
                    module: module__.unwrap_or_default(),
                    type_str: type_str__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    generic_type_params: generic_type_params__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                    is_deleted: is_deleted__.unwrap_or_default(),
                    wsc_index: wsc_index__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.MoveResourceOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for SignatureOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.version != 0 {
            len += 1;
        }
        if !self.signer.is_empty() {
            len += 1;
        }
        if self.is_sender_primary {
            len += 1;
        }
        if !self.signature_type.is_empty() {
            len += 1;
        }
        if !self.public_key.is_empty() {
            len += 1;
        }
        if !self.signature.is_empty() {
            len += 1;
        }
        if self.threshold != 0 {
            len += 1;
        }
        if !self.public_key_indices.is_empty() {
            len += 1;
        }
        if self.multi_agent_index != 0 {
            len += 1;
        }
        if self.multi_sig_index != 0 {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.SignatureOutput", len)?;
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if !self.signer.is_empty() {
            struct_ser.serialize_field("signer", &self.signer)?;
        }
        if self.is_sender_primary {
            struct_ser.serialize_field("isSenderPrimary", &self.is_sender_primary)?;
        }
        if !self.signature_type.is_empty() {
            struct_ser.serialize_field("signatureType", &self.signature_type)?;
        }
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
        if self.threshold != 0 {
            struct_ser.serialize_field("threshold", &self.threshold)?;
        }
        if !self.public_key_indices.is_empty() {
            struct_ser.serialize_field("publicKeyIndices", &self.public_key_indices)?;
        }
        if self.multi_agent_index != 0 {
            struct_ser.serialize_field("multiAgentIndex", &self.multi_agent_index)?;
        }
        if self.multi_sig_index != 0 {
            struct_ser.serialize_field("multiSigIndex", &self.multi_sig_index)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SignatureOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "version",
            "signer",
            "isSenderPrimary",
            "signatureType",
            "publicKey",
            "signature",
            "threshold",
            "publicKeyIndices",
            "multiAgentIndex",
            "multiSigIndex",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Version,
            Signer,
            IsSenderPrimary,
            SignatureType,
            PublicKey,
            Signature,
            Threshold,
            PublicKeyIndices,
            MultiAgentIndex,
            MultiSigIndex,
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
                            "version" => Ok(GeneratedField::Version),
                            "signer" => Ok(GeneratedField::Signer),
                            "isSenderPrimary" => Ok(GeneratedField::IsSenderPrimary),
                            "signatureType" => Ok(GeneratedField::SignatureType),
                            "publicKey" => Ok(GeneratedField::PublicKey),
                            "signature" => Ok(GeneratedField::Signature),
                            "threshold" => Ok(GeneratedField::Threshold),
                            "publicKeyIndices" => Ok(GeneratedField::PublicKeyIndices),
                            "multiAgentIndex" => Ok(GeneratedField::MultiAgentIndex),
                            "multiSigIndex" => Ok(GeneratedField::MultiSigIndex),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SignatureOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.SignatureOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<SignatureOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut version__ = None;
                let mut signer__ = None;
                let mut is_sender_primary__ = None;
                let mut signature_type__ = None;
                let mut public_key__ = None;
                let mut signature__ = None;
                let mut threshold__ = None;
                let mut public_key_indices__ = None;
                let mut multi_agent_index__ = None;
                let mut multi_sig_index__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Signer => {
                            if signer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signer"));
                            }
                            signer__ = Some(map.next_value()?);
                        }
                        GeneratedField::IsSenderPrimary => {
                            if is_sender_primary__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isSenderPrimary"));
                            }
                            is_sender_primary__ = Some(map.next_value()?);
                        }
                        GeneratedField::SignatureType => {
                            if signature_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatureType"));
                            }
                            signature_type__ = Some(map.next_value()?);
                        }
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
                        GeneratedField::MultiAgentIndex => {
                            if multi_agent_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiAgentIndex"));
                            }
                            multi_agent_index__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::MultiSigIndex => {
                            if multi_sig_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiSigIndex"));
                            }
                            multi_sig_index__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                    }
                }
                Ok(SignatureOutput {
                    version: version__.unwrap_or_default(),
                    signer: signer__.unwrap_or_default(),
                    is_sender_primary: is_sender_primary__.unwrap_or_default(),
                    signature_type: signature_type__.unwrap_or_default(),
                    public_key: public_key__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                    threshold: threshold__.unwrap_or_default(),
                    public_key_indices: public_key_indices__.unwrap_or_default(),
                    multi_agent_index: multi_agent_index__.unwrap_or_default(),
                    multi_sig_index: multi_sig_index__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.SignatureOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for TableItemOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.handle.is_empty() {
            len += 1;
        }
        if !self.key.is_empty() {
            len += 1;
        }
        if !self.decoded_key.is_empty() {
            len += 1;
        }
        if !self.key_type.is_empty() {
            len += 1;
        }
        if !self.decoded_value.is_empty() {
            len += 1;
        }
        if !self.value_type.is_empty() {
            len += 1;
        }
        if self.is_deleted {
            len += 1;
        }
        if self.wsc_index != 0 {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.TableItemOutput", len)?;
        if !self.handle.is_empty() {
            struct_ser.serialize_field("handle", &self.handle)?;
        }
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if !self.decoded_key.is_empty() {
            struct_ser.serialize_field("decodedKey", &self.decoded_key)?;
        }
        if !self.key_type.is_empty() {
            struct_ser.serialize_field("keyType", &self.key_type)?;
        }
        if !self.decoded_value.is_empty() {
            struct_ser.serialize_field("decodedValue", &self.decoded_value)?;
        }
        if !self.value_type.is_empty() {
            struct_ser.serialize_field("valueType", &self.value_type)?;
        }
        if self.is_deleted {
            struct_ser.serialize_field("isDeleted", &self.is_deleted)?;
        }
        if self.wsc_index != 0 {
            struct_ser
                .serialize_field("wscIndex", ToString::to_string(&self.wsc_index).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TableItemOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "handle",
            "key",
            "decodedKey",
            "keyType",
            "decodedValue",
            "valueType",
            "isDeleted",
            "wscIndex",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Handle,
            Key,
            DecodedKey,
            KeyType,
            DecodedValue,
            ValueType,
            IsDeleted,
            WscIndex,
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
                            "handle" => Ok(GeneratedField::Handle),
                            "key" => Ok(GeneratedField::Key),
                            "decodedKey" => Ok(GeneratedField::DecodedKey),
                            "keyType" => Ok(GeneratedField::KeyType),
                            "decodedValue" => Ok(GeneratedField::DecodedValue),
                            "valueType" => Ok(GeneratedField::ValueType),
                            "isDeleted" => Ok(GeneratedField::IsDeleted),
                            "wscIndex" => Ok(GeneratedField::WscIndex),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TableItemOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.TableItemOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TableItemOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut handle__ = None;
                let mut key__ = None;
                let mut decoded_key__ = None;
                let mut key_type__ = None;
                let mut decoded_value__ = None;
                let mut value_type__ = None;
                let mut is_deleted__ = None;
                let mut wsc_index__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
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
                        GeneratedField::DecodedKey => {
                            if decoded_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("decodedKey"));
                            }
                            decoded_key__ = Some(map.next_value()?);
                        }
                        GeneratedField::KeyType => {
                            if key_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("keyType"));
                            }
                            key_type__ = Some(map.next_value()?);
                        }
                        GeneratedField::DecodedValue => {
                            if decoded_value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("decodedValue"));
                            }
                            decoded_value__ = Some(map.next_value()?);
                        }
                        GeneratedField::ValueType => {
                            if value_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("valueType"));
                            }
                            value_type__ = Some(map.next_value()?);
                        }
                        GeneratedField::IsDeleted => {
                            if is_deleted__.is_some() {
                                return Err(serde::de::Error::duplicate_field("isDeleted"));
                            }
                            is_deleted__ = Some(map.next_value()?);
                        }
                        GeneratedField::WscIndex => {
                            if wsc_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("wscIndex"));
                            }
                            wsc_index__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                    }
                }
                Ok(TableItemOutput {
                    handle: handle__.unwrap_or_default(),
                    key: key__.unwrap_or_default(),
                    decoded_key: decoded_key__.unwrap_or_default(),
                    key_type: key_type__.unwrap_or_default(),
                    decoded_value: decoded_value__.unwrap_or_default(),
                    value_type: value_type__.unwrap_or_default(),
                    is_deleted: is_deleted__.unwrap_or_default(),
                    wsc_index: wsc_index__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.TableItemOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for TransactionInfoOutput {
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
        if !self.r#type.is_empty() {
            len += 1;
        }
        if self.version != 0 {
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
        if self.epoch != 0 {
            len += 1;
        }
        if self.block_height != 0 {
            len += 1;
        }
        if !self.vm_status.is_empty() {
            len += 1;
        }
        if !self.accumulator_root_hash.is_empty() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.TransactionInfoOutput", len)?;
        if !self.hash.is_empty() {
            struct_ser
                .serialize_field("hash", pbjson::private::base64::encode(&self.hash).as_str())?;
        }
        if !self.r#type.is_empty() {
            struct_ser.serialize_field("type", &self.r#type)?;
        }
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
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
        if self.epoch != 0 {
            struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        }
        if self.block_height != 0 {
            struct_ser.serialize_field(
                "blockHeight",
                ToString::to_string(&self.block_height).as_str(),
            )?;
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
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionInfoOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "hash",
            "type",
            "version",
            "stateChangeHash",
            "eventRootHash",
            "stateCheckpointHash",
            "gasUsed",
            "success",
            "epoch",
            "blockHeight",
            "vmStatus",
            "accumulatorRootHash",
            "timestamp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Hash,
            Type,
            Version,
            StateChangeHash,
            EventRootHash,
            StateCheckpointHash,
            GasUsed,
            Success,
            Epoch,
            BlockHeight,
            VmStatus,
            AccumulatorRootHash,
            Timestamp,
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
                            "type" => Ok(GeneratedField::Type),
                            "version" => Ok(GeneratedField::Version),
                            "stateChangeHash" => Ok(GeneratedField::StateChangeHash),
                            "eventRootHash" => Ok(GeneratedField::EventRootHash),
                            "stateCheckpointHash" => Ok(GeneratedField::StateCheckpointHash),
                            "gasUsed" => Ok(GeneratedField::GasUsed),
                            "success" => Ok(GeneratedField::Success),
                            "epoch" => Ok(GeneratedField::Epoch),
                            "blockHeight" => Ok(GeneratedField::BlockHeight),
                            "vmStatus" => Ok(GeneratedField::VmStatus),
                            "accumulatorRootHash" => Ok(GeneratedField::AccumulatorRootHash),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionInfoOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.TransactionInfoOutput")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<TransactionInfoOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut hash__ = None;
                let mut r#type__ = None;
                let mut version__ = None;
                let mut state_change_hash__ = None;
                let mut event_root_hash__ = None;
                let mut state_checkpoint_hash__ = None;
                let mut gas_used__ = None;
                let mut success__ = None;
                let mut epoch__ = None;
                let mut block_height__ = None;
                let mut vm_status__ = None;
                let mut accumulator_root_hash__ = None;
                let mut timestamp__ = None;
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
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value()?);
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
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(TransactionInfoOutput {
                    hash: hash__.unwrap_or_default(),
                    r#type: r#type__.unwrap_or_default(),
                    version: version__.unwrap_or_default(),
                    state_change_hash: state_change_hash__.unwrap_or_default(),
                    event_root_hash: event_root_hash__.unwrap_or_default(),
                    state_checkpoint_hash: state_checkpoint_hash__,
                    gas_used: gas_used__.unwrap_or_default(),
                    success: success__.unwrap_or_default(),
                    epoch: epoch__.unwrap_or_default(),
                    block_height: block_height__.unwrap_or_default(),
                    vm_status: vm_status__.unwrap_or_default(),
                    accumulator_root_hash: accumulator_root_hash__.unwrap_or_default(),
                    timestamp: timestamp__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.TransactionInfoOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for TransactionOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transaction_info_output.is_some() {
            len += 1;
        }
        if !self.events.is_empty() {
            len += 1;
        }
        if !self.write_set_changes.is_empty() {
            len += 1;
        }
        if self.txn_data.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.TransactionOutput", len)?;
        if let Some(v) = self.transaction_info_output.as_ref() {
            struct_ser.serialize_field("transactionInfoOutput", v)?;
        }
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        if !self.write_set_changes.is_empty() {
            struct_ser.serialize_field("writeSetChanges", &self.write_set_changes)?;
        }
        if let Some(v) = self.txn_data.as_ref() {
            match v {
                transaction_output::TxnData::BlockMetadata(v) => {
                    struct_ser.serialize_field("blockMetadata", v)?;
                }
                transaction_output::TxnData::User(v) => {
                    struct_ser.serialize_field("user", v)?;
                }
                transaction_output::TxnData::Genesis(v) => {
                    struct_ser.serialize_field("genesis", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transactionInfoOutput",
            "events",
            "writeSetChanges",
            "blockMetadata",
            "user",
            "genesis",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionInfoOutput,
            Events,
            WriteSetChanges,
            BlockMetadata,
            User,
            Genesis,
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
                            "transactionInfoOutput" => Ok(GeneratedField::TransactionInfoOutput),
                            "events" => Ok(GeneratedField::Events),
                            "writeSetChanges" => Ok(GeneratedField::WriteSetChanges),
                            "blockMetadata" => Ok(GeneratedField::BlockMetadata),
                            "user" => Ok(GeneratedField::User),
                            "genesis" => Ok(GeneratedField::Genesis),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.TransactionOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut transaction_info_output__ = None;
                let mut events__ = None;
                let mut write_set_changes__ = None;
                let mut txn_data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TransactionInfoOutput => {
                            if transaction_info_output__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "transactionInfoOutput",
                                ));
                            }
                            transaction_info_output__ = Some(map.next_value()?);
                        }
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                        GeneratedField::WriteSetChanges => {
                            if write_set_changes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeSetChanges"));
                            }
                            write_set_changes__ = Some(map.next_value()?);
                        }
                        GeneratedField::BlockMetadata => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockMetadata"));
                            }
                            txn_data__ = Some(transaction_output::TxnData::BlockMetadata(
                                map.next_value()?,
                            ));
                        }
                        GeneratedField::User => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("user"));
                            }
                            txn_data__ = Some(transaction_output::TxnData::User(map.next_value()?));
                        }
                        GeneratedField::Genesis => {
                            if txn_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genesis"));
                            }
                            txn_data__ =
                                Some(transaction_output::TxnData::Genesis(map.next_value()?));
                        }
                    }
                }
                Ok(TransactionOutput {
                    transaction_info_output: transaction_info_output__,
                    events: events__.unwrap_or_default(),
                    write_set_changes: write_set_changes__.unwrap_or_default(),
                    txn_data: txn_data__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.TransactionOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for UserTransactionOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.version != 0 {
            len += 1;
        }
        if !self.parent_signature_type.is_empty() {
            len += 1;
        }
        if !self.sender.is_empty() {
            len += 1;
        }
        if self.sequence_number != 0 {
            len += 1;
        }
        if self.max_gas_amount != 0 {
            len += 1;
        }
        if self.expiration_timestamp_secs.is_some() {
            len += 1;
        }
        if self.gas_unit_price != 0 {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        if !self.signatures.is_empty() {
            len += 1;
        }
        if !self.payload.is_empty() {
            len += 1;
        }
        if !self.entry_function_id_str.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.UserTransactionOutput", len)?;
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if !self.parent_signature_type.is_empty() {
            struct_ser.serialize_field("parentSignatureType", &self.parent_signature_type)?;
        }
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
        if let Some(v) = self.expiration_timestamp_secs.as_ref() {
            struct_ser.serialize_field("expirationTimestampSecs", v)?;
        }
        if self.gas_unit_price != 0 {
            struct_ser.serialize_field(
                "gasUnitPrice",
                ToString::to_string(&self.gas_unit_price).as_str(),
            )?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if !self.signatures.is_empty() {
            struct_ser.serialize_field("signatures", &self.signatures)?;
        }
        if !self.payload.is_empty() {
            struct_ser.serialize_field("payload", &self.payload)?;
        }
        if !self.entry_function_id_str.is_empty() {
            struct_ser.serialize_field("entryFunctionIdStr", &self.entry_function_id_str)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UserTransactionOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "version",
            "parentSignatureType",
            "sender",
            "sequenceNumber",
            "maxGasAmount",
            "expirationTimestampSecs",
            "gasUnitPrice",
            "timestamp",
            "signatures",
            "payload",
            "entryFunctionIdStr",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Version,
            ParentSignatureType,
            Sender,
            SequenceNumber,
            MaxGasAmount,
            ExpirationTimestampSecs,
            GasUnitPrice,
            Timestamp,
            Signatures,
            Payload,
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
                            "version" => Ok(GeneratedField::Version),
                            "parentSignatureType" => Ok(GeneratedField::ParentSignatureType),
                            "sender" => Ok(GeneratedField::Sender),
                            "sequenceNumber" => Ok(GeneratedField::SequenceNumber),
                            "maxGasAmount" => Ok(GeneratedField::MaxGasAmount),
                            "expirationTimestampSecs" => {
                                Ok(GeneratedField::ExpirationTimestampSecs)
                            }
                            "gasUnitPrice" => Ok(GeneratedField::GasUnitPrice),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "signatures" => Ok(GeneratedField::Signatures),
                            "payload" => Ok(GeneratedField::Payload),
                            "entryFunctionIdStr" => Ok(GeneratedField::EntryFunctionIdStr),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UserTransactionOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.UserTransactionOutput")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<UserTransactionOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut version__ = None;
                let mut parent_signature_type__ = None;
                let mut sender__ = None;
                let mut sequence_number__ = None;
                let mut max_gas_amount__ = None;
                let mut expiration_timestamp_secs__ = None;
                let mut gas_unit_price__ = None;
                let mut timestamp__ = None;
                let mut signatures__ = None;
                let mut payload__ = None;
                let mut entry_function_id_str__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::ParentSignatureType => {
                            if parent_signature_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "parentSignatureType",
                                ));
                            }
                            parent_signature_type__ = Some(map.next_value()?);
                        }
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
                        GeneratedField::ExpirationTimestampSecs => {
                            if expiration_timestamp_secs__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "expirationTimestampSecs",
                                ));
                            }
                            expiration_timestamp_secs__ = Some(map.next_value()?);
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
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = Some(map.next_value()?);
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = Some(map.next_value()?);
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = Some(map.next_value()?);
                        }
                        GeneratedField::EntryFunctionIdStr => {
                            if entry_function_id_str__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "entryFunctionIdStr",
                                ));
                            }
                            entry_function_id_str__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(UserTransactionOutput {
                    version: version__.unwrap_or_default(),
                    parent_signature_type: parent_signature_type__.unwrap_or_default(),
                    sender: sender__.unwrap_or_default(),
                    sequence_number: sequence_number__.unwrap_or_default(),
                    max_gas_amount: max_gas_amount__.unwrap_or_default(),
                    expiration_timestamp_secs: expiration_timestamp_secs__,
                    gas_unit_price: gas_unit_price__.unwrap_or_default(),
                    timestamp: timestamp__,
                    signatures: signatures__.unwrap_or_default(),
                    payload: payload__.unwrap_or_default(),
                    entry_function_id_str: entry_function_id_str__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.UserTransactionOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for WriteSetChangeOutput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.version != 0 {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        if !self.r#type.is_empty() {
            len += 1;
        }
        if self.change.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.block_output.v1.WriteSetChangeOutput", len)?;
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if !self.hash.is_empty() {
            struct_ser
                .serialize_field("hash", pbjson::private::base64::encode(&self.hash).as_str())?;
        }
        if !self.r#type.is_empty() {
            struct_ser.serialize_field("type", &self.r#type)?;
        }
        if let Some(v) = self.change.as_ref() {
            match v {
                write_set_change_output::Change::MoveModule(v) => {
                    struct_ser.serialize_field("moveModule", v)?;
                }
                write_set_change_output::Change::MoveResource(v) => {
                    struct_ser.serialize_field("moveResource", v)?;
                }
                write_set_change_output::Change::TableItem(v) => {
                    struct_ser.serialize_field("tableItem", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for WriteSetChangeOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "version",
            "hash",
            "type",
            "moveModule",
            "moveResource",
            "tableItem",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Version,
            Hash,
            Type,
            MoveModule,
            MoveResource,
            TableItem,
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
                            "version" => Ok(GeneratedField::Version),
                            "hash" => Ok(GeneratedField::Hash),
                            "type" => Ok(GeneratedField::Type),
                            "moveModule" => Ok(GeneratedField::MoveModule),
                            "moveResource" => Ok(GeneratedField::MoveResource),
                            "tableItem" => Ok(GeneratedField::TableItem),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = WriteSetChangeOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.block_output.v1.WriteSetChangeOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<WriteSetChangeOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut version__ = None;
                let mut hash__ = None;
                let mut r#type__ = None;
                let mut change__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(
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
                        GeneratedField::MoveModule => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("moveModule"));
                            }
                            change__ = Some(write_set_change_output::Change::MoveModule(
                                map.next_value()?,
                            ));
                        }
                        GeneratedField::MoveResource => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("moveResource"));
                            }
                            change__ = Some(write_set_change_output::Change::MoveResource(
                                map.next_value()?,
                            ));
                        }
                        GeneratedField::TableItem => {
                            if change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tableItem"));
                            }
                            change__ = Some(write_set_change_output::Change::TableItem(
                                map.next_value()?,
                            ));
                        }
                    }
                }
                Ok(WriteSetChangeOutput {
                    version: version__.unwrap_or_default(),
                    hash: hash__.unwrap_or_default(),
                    r#type: r#type__.unwrap_or_default(),
                    change: change__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.block_output.v1.WriteSetChangeOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
