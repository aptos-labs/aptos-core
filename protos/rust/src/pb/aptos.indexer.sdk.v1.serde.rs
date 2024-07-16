// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// @generated
impl serde::Serialize for SdkEventsStepRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transaction_context.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.sdk.v1.SdkEventsStepRequest", len)?;
        if let Some(v) = self.transaction_context.as_ref() {
            struct_ser.serialize_field("transactionContext", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SdkEventsStepRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction_context",
            "transactionContext",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionContext,
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
                            "transactionContext" | "transaction_context" => Ok(GeneratedField::TransactionContext),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SdkEventsStepRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.sdk.v1.SdkEventsStepRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<SdkEventsStepRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transaction_context__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TransactionContext => {
                            if transaction_context__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionContext"));
                            }
                            transaction_context__ = map.next_value()?;
                        }
                    }
                }
                Ok(SdkEventsStepRequest {
                    transaction_context: transaction_context__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.sdk.v1.SdkEventsStepRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SdkEventsStepResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.start_version != 0 {
            len += 1;
        }
        if self.end_version != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.sdk.v1.SdkEventsStepResponse", len)?;
        if self.start_version != 0 {
            struct_ser.serialize_field("startVersion", ToString::to_string(&self.start_version).as_str())?;
        }
        if self.end_version != 0 {
            struct_ser.serialize_field("endVersion", ToString::to_string(&self.end_version).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SdkEventsStepResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_version",
            "startVersion",
            "end_version",
            "endVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartVersion,
            EndVersion,
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
                            "startVersion" | "start_version" => Ok(GeneratedField::StartVersion),
                            "endVersion" | "end_version" => Ok(GeneratedField::EndVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SdkEventsStepResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.sdk.v1.SdkEventsStepResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<SdkEventsStepResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_version__ = None;
                let mut end_version__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::StartVersion => {
                            if start_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startVersion"));
                            }
                            start_version__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EndVersion => {
                            if end_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endVersion"));
                            }
                            end_version__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(SdkEventsStepResponse {
                    start_version: start_version__.unwrap_or_default(),
                    end_version: end_version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.sdk.v1.SdkEventsStepResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionContext {
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
        if self.start_version != 0 {
            len += 1;
        }
        if self.end_version != 0 {
            len += 1;
        }
        if self.start_transaction_timestamp.is_some() {
            len += 1;
        }
        if self.end_transaction_timestamp.is_some() {
            len += 1;
        }
        if self.total_size_in_bytes != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.sdk.v1.TransactionContext", len)?;
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        if self.start_version != 0 {
            struct_ser.serialize_field("startVersion", ToString::to_string(&self.start_version).as_str())?;
        }
        if self.end_version != 0 {
            struct_ser.serialize_field("endVersion", ToString::to_string(&self.end_version).as_str())?;
        }
        if let Some(v) = self.start_transaction_timestamp.as_ref() {
            struct_ser.serialize_field("startTransactionTimestamp", v)?;
        }
        if let Some(v) = self.end_transaction_timestamp.as_ref() {
            struct_ser.serialize_field("endTransactionTimestamp", v)?;
        }
        if self.total_size_in_bytes != 0 {
            struct_ser.serialize_field("totalSizeInBytes", ToString::to_string(&self.total_size_in_bytes).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionContext {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "events",
            "start_version",
            "startVersion",
            "end_version",
            "endVersion",
            "start_transaction_timestamp",
            "startTransactionTimestamp",
            "end_transaction_timestamp",
            "endTransactionTimestamp",
            "total_size_in_bytes",
            "totalSizeInBytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Events,
            StartVersion,
            EndVersion,
            StartTransactionTimestamp,
            EndTransactionTimestamp,
            TotalSizeInBytes,
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
                            "startVersion" | "start_version" => Ok(GeneratedField::StartVersion),
                            "endVersion" | "end_version" => Ok(GeneratedField::EndVersion),
                            "startTransactionTimestamp" | "start_transaction_timestamp" => Ok(GeneratedField::StartTransactionTimestamp),
                            "endTransactionTimestamp" | "end_transaction_timestamp" => Ok(GeneratedField::EndTransactionTimestamp),
                            "totalSizeInBytes" | "total_size_in_bytes" => Ok(GeneratedField::TotalSizeInBytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionContext;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.sdk.v1.TransactionContext")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionContext, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut events__ = None;
                let mut start_version__ = None;
                let mut end_version__ = None;
                let mut start_transaction_timestamp__ = None;
                let mut end_transaction_timestamp__ = None;
                let mut total_size_in_bytes__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                        GeneratedField::StartVersion => {
                            if start_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startVersion"));
                            }
                            start_version__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EndVersion => {
                            if end_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endVersion"));
                            }
                            end_version__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StartTransactionTimestamp => {
                            if start_transaction_timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTransactionTimestamp"));
                            }
                            start_transaction_timestamp__ = map.next_value()?;
                        }
                        GeneratedField::EndTransactionTimestamp => {
                            if end_transaction_timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endTransactionTimestamp"));
                            }
                            end_transaction_timestamp__ = map.next_value()?;
                        }
                        GeneratedField::TotalSizeInBytes => {
                            if total_size_in_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalSizeInBytes"));
                            }
                            total_size_in_bytes__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(TransactionContext {
                    events: events__.unwrap_or_default(),
                    start_version: start_version__.unwrap_or_default(),
                    end_version: end_version__.unwrap_or_default(),
                    start_transaction_timestamp: start_transaction_timestamp__,
                    end_transaction_timestamp: end_transaction_timestamp__,
                    total_size_in_bytes: total_size_in_bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.sdk.v1.TransactionContext", FIELDS, GeneratedVisitor)
    }
}
