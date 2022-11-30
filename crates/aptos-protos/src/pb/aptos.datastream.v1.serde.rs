// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// @generated
impl serde::Serialize for RawDatastreamRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.starting_version != 0 {
            len += 1;
        }
        if self.fetcher_count != 0 {
            len += 1;
        }
        if self.processor_task_count != 0 {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.datastream.v1.RawDatastreamRequest", len)?;
        if self.starting_version != 0 {
            struct_ser.serialize_field(
                "startingVersion",
                ToString::to_string(&self.starting_version).as_str(),
            )?;
        }
        if self.fetcher_count != 0 {
            struct_ser.serialize_field(
                "fetcherCount",
                ToString::to_string(&self.fetcher_count).as_str(),
            )?;
        }
        if self.processor_task_count != 0 {
            struct_ser.serialize_field(
                "processorTaskCount",
                ToString::to_string(&self.processor_task_count).as_str(),
            )?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RawDatastreamRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "starting_version",
            "startingVersion",
            "fetcher_count",
            "fetcherCount",
            "processor_task_count",
            "processorTaskCount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartingVersion,
            FetcherCount,
            ProcessorTaskCount,
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
                            "startingVersion" | "starting_version" => {
                                Ok(GeneratedField::StartingVersion)
                            }
                            "fetcherCount" | "fetcher_count" => Ok(GeneratedField::FetcherCount),
                            "processorTaskCount" | "processor_task_count" => {
                                Ok(GeneratedField::ProcessorTaskCount)
                            }
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RawDatastreamRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.datastream.v1.RawDatastreamRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<RawDatastreamRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut starting_version__ = None;
                let mut fetcher_count__ = None;
                let mut processor_task_count__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::StartingVersion => {
                            if starting_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startingVersion"));
                            }
                            starting_version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::FetcherCount => {
                            if fetcher_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fetcherCount"));
                            }
                            fetcher_count__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::ProcessorTaskCount => {
                            if processor_task_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "processorTaskCount",
                                ));
                            }
                            processor_task_count__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                    }
                }
                Ok(RawDatastreamRequest {
                    starting_version: starting_version__.unwrap_or_default(),
                    fetcher_count: fetcher_count__.unwrap_or_default(),
                    processor_task_count: processor_task_count__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.datastream.v1.RawDatastreamRequest",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for RawDatastreamResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.size != 0 {
            len += 1;
        }
        if self.data.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.datastream.v1.RawDatastreamResponse", len)?;
        if self.size != 0 {
            struct_ser.serialize_field("size", ToString::to_string(&self.size).as_str())?;
        }
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RawDatastreamResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["size", "data"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Size,
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
                            "size" => Ok(GeneratedField::Size),
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
            type Value = RawDatastreamResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.datastream.v1.RawDatastreamResponse")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<RawDatastreamResponse, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut size__ = None;
                let mut data__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Size => {
                            if size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("size"));
                            }
                            size__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map.next_value()?;
                        }
                    }
                }
                Ok(RawDatastreamResponse {
                    size: size__.unwrap_or_default(),
                    data: data__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.datastream.v1.RawDatastreamResponse",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for TransactionsData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.transactions.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.datastream.v1.TransactionsData", len)?;
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionsData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["transactions"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transactions,
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
                            "transactions" => Ok(GeneratedField::Transactions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionsData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.datastream.v1.TransactionsData")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionsData, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut transactions__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(TransactionsData {
                    transactions: transactions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.datastream.v1.TransactionsData",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for transactions_data::TransactionData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.encoded_proto_data.is_empty() {
            len += 1;
        }
        if self.version != 0 {
            len += 1;
        }
        let mut struct_ser = serializer
            .serialize_struct("aptos.datastream.v1.TransactionsData.TransactionData", len)?;
        if !self.encoded_proto_data.is_empty() {
            struct_ser.serialize_field("encodedProtoData", &self.encoded_proto_data)?;
        }
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for transactions_data::TransactionData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["encoded_proto_data", "encodedProtoData", "version"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            EncodedProtoData,
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
                            "encodedProtoData" | "encoded_proto_data" => {
                                Ok(GeneratedField::EncodedProtoData)
                            }
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
            type Value = transactions_data::TransactionData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.datastream.v1.TransactionsData.TransactionData")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> std::result::Result<transactions_data::TransactionData, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut encoded_proto_data__ = None;
                let mut version__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::EncodedProtoData => {
                            if encoded_proto_data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("encodedProtoData"));
                            }
                            encoded_proto_data__ = Some(map.next_value()?);
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
                Ok(transactions_data::TransactionData {
                    encoded_proto_data: encoded_proto_data__.unwrap_or_default(),
                    version: version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.datastream.v1.TransactionsData.TransactionData",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
