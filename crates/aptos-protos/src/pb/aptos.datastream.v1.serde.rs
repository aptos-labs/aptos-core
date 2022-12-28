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
        if self.processor_task_count != 0 {
            len += 1;
        }
        if self.processor_batch_size != 0 {
            len += 1;
        }
        if self.output_batch_size != 0 {
            len += 1;
        }
        if self.chain_id != 0 {
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
        if self.processor_task_count != 0 {
            struct_ser.serialize_field(
                "processorTaskCount",
                ToString::to_string(&self.processor_task_count).as_str(),
            )?;
        }
        if self.processor_batch_size != 0 {
            struct_ser.serialize_field(
                "processorBatchSize",
                ToString::to_string(&self.processor_batch_size).as_str(),
            )?;
        }
        if self.output_batch_size != 0 {
            struct_ser.serialize_field(
                "outputBatchSize",
                ToString::to_string(&self.output_batch_size).as_str(),
            )?;
        }
        if self.chain_id != 0 {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
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
            "processor_task_count",
            "processorTaskCount",
            "processor_batch_size",
            "processorBatchSize",
            "output_batch_size",
            "outputBatchSize",
            "chain_id",
            "chainId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartingVersion,
            ProcessorTaskCount,
            ProcessorBatchSize,
            OutputBatchSize,
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
                            "startingVersion" | "starting_version" => {
                                Ok(GeneratedField::StartingVersion)
                            }
                            "processorTaskCount" | "processor_task_count" => {
                                Ok(GeneratedField::ProcessorTaskCount)
                            }
                            "processorBatchSize" | "processor_batch_size" => {
                                Ok(GeneratedField::ProcessorBatchSize)
                            }
                            "outputBatchSize" | "output_batch_size" => {
                                Ok(GeneratedField::OutputBatchSize)
                            }
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
            type Value = RawDatastreamRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.datastream.v1.RawDatastreamRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<RawDatastreamRequest, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut starting_version__ = None;
                let mut processor_task_count__ = None;
                let mut processor_batch_size__ = None;
                let mut output_batch_size__ = None;
                let mut chain_id__ = None;
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
                        GeneratedField::ProcessorBatchSize => {
                            if processor_batch_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "processorBatchSize",
                                ));
                            }
                            processor_batch_size__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::OutputBatchSize => {
                            if output_batch_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outputBatchSize"));
                            }
                            output_batch_size__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
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
                Ok(RawDatastreamRequest {
                    starting_version: starting_version__.unwrap_or_default(),
                    processor_task_count: processor_task_count__.unwrap_or_default(),
                    processor_batch_size: processor_batch_size__.unwrap_or_default(),
                    output_batch_size: output_batch_size__.unwrap_or_default(),
                    chain_id: chain_id__.unwrap_or_default(),
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
        if self.response.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.datastream.v1.RawDatastreamResponse", len)?;
        if let Some(v) = self.response.as_ref() {
            match v {
                raw_datastream_response::Response::Status(v) => {
                    struct_ser.serialize_field("status", v)?;
                }
                raw_datastream_response::Response::Data(v) => {
                    struct_ser.serialize_field("data", v)?;
                }
            }
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
        const FIELDS: &[&str] = &["status", "data"];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Status,
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
                            "status" => Ok(GeneratedField::Status),
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
                let mut response__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Status => {
                            if response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("status"));
                            }
                            response__ = map
                                .next_value::<::std::option::Option<_>>()?
                                .map(raw_datastream_response::Response::Status);
                        }
                        GeneratedField::Data => {
                            if response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            response__ = map
                                .next_value::<::std::option::Option<_>>()?
                                .map(raw_datastream_response::Response::Data);
                        }
                    }
                }
                Ok(RawDatastreamResponse {
                    response: response__,
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
impl serde::Serialize for raw_datastream_response::ResponseType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Status => "STATUS",
            Self::Data => "DATA",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for raw_datastream_response::ResponseType {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["STATUS", "DATA"];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = raw_datastream_response::ResponseType;

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
                    .and_then(raw_datastream_response::ResponseType::from_i32)
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
                    .and_then(raw_datastream_response::ResponseType::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "STATUS" => Ok(raw_datastream_response::ResponseType::Status),
                    "DATA" => Ok(raw_datastream_response::ResponseType::Data),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for StreamStatus {
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
        if self.start_version != 0 {
            len += 1;
        }
        if self.end_version.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.datastream.v1.StreamStatus", len)?;
        if self.r#type != 0 {
            let v = stream_status::StatusType::from_i32(self.r#type).ok_or_else(|| {
                serde::ser::Error::custom(format!("Invalid variant {}", self.r#type))
            })?;
            struct_ser.serialize_field("type", &v)?;
        }
        if self.start_version != 0 {
            struct_ser.serialize_field(
                "startVersion",
                ToString::to_string(&self.start_version).as_str(),
            )?;
        }
        if let Some(v) = self.end_version.as_ref() {
            struct_ser.serialize_field("endVersion", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamStatus {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "start_version",
            "startVersion",
            "end_version",
            "endVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
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
            type Value = StreamStatus;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.datastream.v1.StreamStatus")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<StreamStatus, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut start_version__ = None;
                let mut end_version__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map.next_value::<stream_status::StatusType>()? as i32);
                        }
                        GeneratedField::StartVersion => {
                            if start_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startVersion"));
                            }
                            start_version__ = Some(
                                map.next_value::<::pbjson::private::NumberDeserialize<_>>()?
                                    .0,
                            );
                        }
                        GeneratedField::EndVersion => {
                            if end_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endVersion"));
                            }
                            end_version__ =
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(StreamStatus {
                    r#type: r#type__.unwrap_or_default(),
                    start_version: start_version__.unwrap_or_default(),
                    end_version: end_version__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.datastream.v1.StreamStatus",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for stream_status::StatusType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Init => "INIT",
            Self::BatchEnd => "BATCH_END",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for stream_status::StatusType {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["INIT", "BATCH_END"];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = stream_status::StatusType;

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
                    .and_then(stream_status::StatusType::from_i32)
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
                    .and_then(stream_status::StatusType::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "INIT" => Ok(stream_status::StatusType::Init),
                    "BATCH_END" => Ok(stream_status::StatusType::BatchEnd),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
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
        if !self.encoded_proto_data.is_empty() {
            len += 1;
        }
        if self.version != 0 {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.datastream.v1.TransactionOutput", len)?;
        if !self.encoded_proto_data.is_empty() {
            struct_ser.serialize_field("encodedProtoData", &self.encoded_proto_data)?;
        }
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
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
            type Value = TransactionOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.datastream.v1.TransactionOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionOutput, V::Error>
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
                Ok(TransactionOutput {
                    encoded_proto_data: encoded_proto_data__.unwrap_or_default(),
                    version: version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.datastream.v1.TransactionOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
impl serde::Serialize for TransactionsOutput {
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
        if self.transactions_timestamp.is_some() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("aptos.datastream.v1.TransactionsOutput", len)?;
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        if let Some(v) = self.transactions_timestamp.as_ref() {
            struct_ser.serialize_field("transactionsTimestamp", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionsOutput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transactions",
            "transactions_timestamp",
            "transactionsTimestamp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transactions,
            TransactionsTimestamp,
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
                            "transactionsTimestamp" | "transactions_timestamp" => {
                                Ok(GeneratedField::TransactionsTimestamp)
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
            type Value = TransactionsOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.datastream.v1.TransactionsOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionsOutput, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut transactions__ = None;
                let mut transactions_timestamp__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ = Some(map.next_value()?);
                        }
                        GeneratedField::TransactionsTimestamp => {
                            if transactions_timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "transactionsTimestamp",
                                ));
                            }
                            transactions_timestamp__ = map.next_value()?;
                        }
                    }
                }
                Ok(TransactionsOutput {
                    transactions: transactions__.unwrap_or_default(),
                    transactions_timestamp: transactions_timestamp__,
                })
            }
        }
        deserializer.deserialize_struct(
            "aptos.datastream.v1.TransactionsOutput",
            FIELDS,
            GeneratedVisitor,
        )
    }
}
