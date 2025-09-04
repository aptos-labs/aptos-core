// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// @generated
impl serde::Serialize for GetTransactionsFromNodeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.starting_version.is_some() {
            len += 1;
        }
        if self.transactions_count.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.internal.fullnode.v1.GetTransactionsFromNodeRequest", len)?;
        if let Some(v) = self.starting_version.as_ref() {
            struct_ser.serialize_field("startingVersion", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.transactions_count.as_ref() {
            struct_ser.serialize_field("transactionsCount", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionsFromNodeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "starting_version",
            "startingVersion",
            "transactions_count",
            "transactionsCount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartingVersion,
            TransactionsCount,
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
                            "startingVersion" | "starting_version" => Ok(GeneratedField::StartingVersion),
                            "transactionsCount" | "transactions_count" => Ok(GeneratedField::TransactionsCount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionsFromNodeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.internal.fullnode.v1.GetTransactionsFromNodeRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetTransactionsFromNodeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut starting_version__ = None;
                let mut transactions_count__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::StartingVersion => {
                            if starting_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startingVersion"));
                            }
                            starting_version__ =
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::TransactionsCount => {
                            if transactions_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionsCount"));
                            }
                            transactions_count__ =
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(GetTransactionsFromNodeRequest {
                    starting_version: starting_version__,
                    transactions_count: transactions_count__,
                })
            }
        }
        deserializer.deserialize_struct("velor.internal.fullnode.v1.GetTransactionsFromNodeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PingFullnodeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("velor.internal.fullnode.v1.PingFullnodeRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PingFullnodeRequest {
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
            type Value = PingFullnodeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.internal.fullnode.v1.PingFullnodeRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PingFullnodeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(PingFullnodeRequest {
                })
            }
        }
        deserializer.deserialize_struct("velor.internal.fullnode.v1.PingFullnodeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PingFullnodeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.info.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.internal.fullnode.v1.PingFullnodeResponse", len)?;
        if let Some(v) = self.info.as_ref() {
            struct_ser.serialize_field("info", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PingFullnodeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "info",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Info,
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
                            "info" => Ok(GeneratedField::Info),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PingFullnodeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.internal.fullnode.v1.PingFullnodeResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PingFullnodeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut info__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Info => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("info"));
                            }
                            info__ = map.next_value()?;
                        }
                    }
                }
                Ok(PingFullnodeResponse {
                    info: info__,
                })
            }
        }
        deserializer.deserialize_struct("velor.internal.fullnode.v1.PingFullnodeResponse", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("velor.internal.fullnode.v1.StreamStatus", len)?;
        if self.r#type != 0 {
            let v = stream_status::StatusType::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if self.start_version != 0 {
            struct_ser.serialize_field("startVersion", ToString::to_string(&self.start_version).as_str())?;
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
                formatter.write_str("struct velor.internal.fullnode.v1.StreamStatus")
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
                            start_version__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
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
        deserializer.deserialize_struct("velor.internal.fullnode.v1.StreamStatus", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for stream_status::StatusType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "STATUS_TYPE_UNSPECIFIED",
            Self::Init => "STATUS_TYPE_INIT",
            Self::BatchEnd => "STATUS_TYPE_BATCH_END",
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
        const FIELDS: &[&str] = &[
            "STATUS_TYPE_UNSPECIFIED",
            "STATUS_TYPE_INIT",
            "STATUS_TYPE_BATCH_END",
        ];

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
                    "STATUS_TYPE_UNSPECIFIED" => Ok(stream_status::StatusType::Unspecified),
                    "STATUS_TYPE_INIT" => Ok(stream_status::StatusType::Init),
                    "STATUS_TYPE_BATCH_END" => Ok(stream_status::StatusType::BatchEnd),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionsFromNodeResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.chain_id != 0 {
            len += 1;
        }
        if self.response.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.internal.fullnode.v1.TransactionsFromNodeResponse", len)?;
        if self.chain_id != 0 {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        if let Some(v) = self.response.as_ref() {
            match v {
                transactions_from_node_response::Response::Status(v) => {
                    struct_ser.serialize_field("status", v)?;
                }
                transactions_from_node_response::Response::Data(v) => {
                    struct_ser.serialize_field("data", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionsFromNodeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_id",
            "chainId",
            "status",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainId,
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

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
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
            type Value = TransactionsFromNodeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.internal.fullnode.v1.TransactionsFromNodeResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionsFromNodeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_id__ = None;
                let mut response__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Status => {
                            if response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("status"));
                            }
                            response__ = map.next_value::<::std::option::Option<_>>()?.map(transactions_from_node_response::Response::Status)
;
                        }
                        GeneratedField::Data => {
                            if response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            response__ = map.next_value::<::std::option::Option<_>>()?.map(transactions_from_node_response::Response::Data)
;
                        }
                    }
                }
                Ok(TransactionsFromNodeResponse {
                    chain_id: chain_id__.unwrap_or_default(),
                    response: response__,
                })
            }
        }
        deserializer.deserialize_struct("velor.internal.fullnode.v1.TransactionsFromNodeResponse", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("velor.internal.fullnode.v1.TransactionsOutput", len)?;
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
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
        ];

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

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            type Value = TransactionsOutput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct velor.internal.fullnode.v1.TransactionsOutput")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionsOutput, V::Error>
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
                Ok(TransactionsOutput {
                    transactions: transactions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("velor.internal.fullnode.v1.TransactionsOutput", FIELDS, GeneratedVisitor)
    }
}
