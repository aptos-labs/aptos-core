// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// @generated
impl serde::Serialize for Empty {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("aptos.remote_executor.v1.Empty", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Empty {
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
            type Value = Empty;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.remote_executor.v1.Empty")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Empty, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(Empty {
                })
            }
        }
        deserializer.deserialize_struct("aptos.remote_executor.v1.Empty", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NetworkMessage {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.message.is_empty() {
            len += 1;
        }
        if !self.message_type.is_empty() {
            len += 1;
        }
        if self.ms_since_epoch.is_some() {
            len += 1;
        }
        if self.seq_no.is_some() {
            len += 1;
        }
        if self.shard_id.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.remote_executor.v1.NetworkMessage", len)?;
        if !self.message.is_empty() {
            struct_ser.serialize_field("message", pbjson::private::base64::encode(&self.message).as_str())?;
        }
        if !self.message_type.is_empty() {
            struct_ser.serialize_field("messageType", &self.message_type)?;
        }
        if let Some(v) = self.ms_since_epoch.as_ref() {
            struct_ser.serialize_field("msSinceEpoch", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.seq_no.as_ref() {
            struct_ser.serialize_field("seqNo", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.shard_id.as_ref() {
            struct_ser.serialize_field("shardId", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NetworkMessage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "message",
            "message_type",
            "messageType",
            "ms_since_epoch",
            "msSinceEpoch",
            "seq_no",
            "seqNo",
            "shard_id",
            "shardId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Message,
            MessageType,
            MsSinceEpoch,
            SeqNo,
            ShardId,
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
                            "message" => Ok(GeneratedField::Message),
                            "messageType" | "message_type" => Ok(GeneratedField::MessageType),
                            "msSinceEpoch" | "ms_since_epoch" => Ok(GeneratedField::MsSinceEpoch),
                            "seqNo" | "seq_no" => Ok(GeneratedField::SeqNo),
                            "shardId" | "shard_id" => Ok(GeneratedField::ShardId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NetworkMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.remote_executor.v1.NetworkMessage")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<NetworkMessage, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut message__ = None;
                let mut message_type__ = None;
                let mut ms_since_epoch__ = None;
                let mut seq_no__ = None;
                let mut shard_id__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Message => {
                            if message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MessageType => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messageType"));
                            }
                            message_type__ = Some(map.next_value()?);
                        }
                        GeneratedField::MsSinceEpoch => {
                            if ms_since_epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("msSinceEpoch"));
                            }
                            ms_since_epoch__ =
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::SeqNo => {
                            if seq_no__.is_some() {
                                return Err(serde::de::Error::duplicate_field("seqNo"));
                            }
                            seq_no__ =
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::ShardId => {
                            if shard_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("shardId"));
                            }
                            shard_id__ =
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(NetworkMessage {
                    message: message__.unwrap_or_default(),
                    message_type: message_type__.unwrap_or_default(),
                    ms_since_epoch: ms_since_epoch__,
                    seq_no: seq_no__,
                    shard_id: shard_id__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.remote_executor.v1.NetworkMessage", FIELDS, GeneratedVisitor)
    }
}
