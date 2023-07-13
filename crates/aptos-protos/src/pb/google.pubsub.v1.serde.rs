// Copyright © Aptos Foundation

// @generated
impl serde::Serialize for AcknowledgeRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        if !self.ack_ids.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.AcknowledgeRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        if !self.ack_ids.is_empty() {
            struct_ser.serialize_field("ackIds", &self.ack_ids)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AcknowledgeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
            "ack_ids",
            "ackIds",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
            AckIds,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            "ackIds" | "ack_ids" => Ok(GeneratedField::AckIds),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AcknowledgeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.AcknowledgeRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<AcknowledgeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                let mut ack_ids__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                        GeneratedField::AckIds => {
                            if ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ackIds"));
                            }
                            ack_ids__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(AcknowledgeRequest {
                    subscription: subscription__.unwrap_or_default(),
                    ack_ids: ack_ids__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.AcknowledgeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BigQueryConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.table.is_empty() {
            len += 1;
        }
        if self.use_topic_schema {
            len += 1;
        }
        if self.write_metadata {
            len += 1;
        }
        if self.drop_unknown_fields {
            len += 1;
        }
        if self.state != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.BigQueryConfig", len)?;
        if !self.table.is_empty() {
            struct_ser.serialize_field("table", &self.table)?;
        }
        if self.use_topic_schema {
            struct_ser.serialize_field("useTopicSchema", &self.use_topic_schema)?;
        }
        if self.write_metadata {
            struct_ser.serialize_field("writeMetadata", &self.write_metadata)?;
        }
        if self.drop_unknown_fields {
            struct_ser.serialize_field("dropUnknownFields", &self.drop_unknown_fields)?;
        }
        if self.state != 0 {
            let v = big_query_config::State::from_i32(self.state)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.state)))?;
            struct_ser.serialize_field("state", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BigQueryConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "table",
            "use_topic_schema",
            "useTopicSchema",
            "write_metadata",
            "writeMetadata",
            "drop_unknown_fields",
            "dropUnknownFields",
            "state",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Table,
            UseTopicSchema,
            WriteMetadata,
            DropUnknownFields,
            State,
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
                            "table" => Ok(GeneratedField::Table),
                            "useTopicSchema" | "use_topic_schema" => Ok(GeneratedField::UseTopicSchema),
                            "writeMetadata" | "write_metadata" => Ok(GeneratedField::WriteMetadata),
                            "dropUnknownFields" | "drop_unknown_fields" => Ok(GeneratedField::DropUnknownFields),
                            "state" => Ok(GeneratedField::State),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BigQueryConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.BigQueryConfig")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<BigQueryConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut table__ = None;
                let mut use_topic_schema__ = None;
                let mut write_metadata__ = None;
                let mut drop_unknown_fields__ = None;
                let mut state__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Table => {
                            if table__.is_some() {
                                return Err(serde::de::Error::duplicate_field("table"));
                            }
                            table__ = Some(map.next_value()?);
                        }
                        GeneratedField::UseTopicSchema => {
                            if use_topic_schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("useTopicSchema"));
                            }
                            use_topic_schema__ = Some(map.next_value()?);
                        }
                        GeneratedField::WriteMetadata => {
                            if write_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeMetadata"));
                            }
                            write_metadata__ = Some(map.next_value()?);
                        }
                        GeneratedField::DropUnknownFields => {
                            if drop_unknown_fields__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dropUnknownFields"));
                            }
                            drop_unknown_fields__ = Some(map.next_value()?);
                        }
                        GeneratedField::State => {
                            if state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state__ = Some(map.next_value::<big_query_config::State>()? as i32);
                        }
                    }
                }
                Ok(BigQueryConfig {
                    table: table__.unwrap_or_default(),
                    use_topic_schema: use_topic_schema__.unwrap_or_default(),
                    write_metadata: write_metadata__.unwrap_or_default(),
                    drop_unknown_fields: drop_unknown_fields__.unwrap_or_default(),
                    state: state__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.BigQueryConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for big_query_config::State {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "STATE_UNSPECIFIED",
            Self::Active => "ACTIVE",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::NotFound => "NOT_FOUND",
            Self::SchemaMismatch => "SCHEMA_MISMATCH",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for big_query_config::State {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "STATE_UNSPECIFIED",
            "ACTIVE",
            "PERMISSION_DENIED",
            "NOT_FOUND",
            "SCHEMA_MISMATCH",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = big_query_config::State;

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
                    .and_then(big_query_config::State::from_i32)
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
                    .and_then(big_query_config::State::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "STATE_UNSPECIFIED" => Ok(big_query_config::State::Unspecified),
                    "ACTIVE" => Ok(big_query_config::State::Active),
                    "PERMISSION_DENIED" => Ok(big_query_config::State::PermissionDenied),
                    "NOT_FOUND" => Ok(big_query_config::State::NotFound),
                    "SCHEMA_MISMATCH" => Ok(big_query_config::State::SchemaMismatch),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for CloudStorageConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.bucket.is_empty() {
            len += 1;
        }
        if !self.filename_prefix.is_empty() {
            len += 1;
        }
        if !self.filename_suffix.is_empty() {
            len += 1;
        }
        if self.max_duration.is_some() {
            len += 1;
        }
        if self.max_bytes != 0 {
            len += 1;
        }
        if self.state != 0 {
            len += 1;
        }
        if self.output_format.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.CloudStorageConfig", len)?;
        if !self.bucket.is_empty() {
            struct_ser.serialize_field("bucket", &self.bucket)?;
        }
        if !self.filename_prefix.is_empty() {
            struct_ser.serialize_field("filenamePrefix", &self.filename_prefix)?;
        }
        if !self.filename_suffix.is_empty() {
            struct_ser.serialize_field("filenameSuffix", &self.filename_suffix)?;
        }
        if let Some(v) = self.max_duration.as_ref() {
            struct_ser.serialize_field("maxDuration", v)?;
        }
        if self.max_bytes != 0 {
            struct_ser.serialize_field("maxBytes", ToString::to_string(&self.max_bytes).as_str())?;
        }
        if self.state != 0 {
            let v = cloud_storage_config::State::from_i32(self.state)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.state)))?;
            struct_ser.serialize_field("state", &v)?;
        }
        if let Some(v) = self.output_format.as_ref() {
            match v {
                cloud_storage_config::OutputFormat::TextConfig(v) => {
                    struct_ser.serialize_field("textConfig", v)?;
                }
                cloud_storage_config::OutputFormat::AvroConfig(v) => {
                    struct_ser.serialize_field("avroConfig", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CloudStorageConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bucket",
            "filename_prefix",
            "filenamePrefix",
            "filename_suffix",
            "filenameSuffix",
            "max_duration",
            "maxDuration",
            "max_bytes",
            "maxBytes",
            "state",
            "text_config",
            "textConfig",
            "avro_config",
            "avroConfig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bucket,
            FilenamePrefix,
            FilenameSuffix,
            MaxDuration,
            MaxBytes,
            State,
            TextConfig,
            AvroConfig,
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
                            "bucket" => Ok(GeneratedField::Bucket),
                            "filenamePrefix" | "filename_prefix" => Ok(GeneratedField::FilenamePrefix),
                            "filenameSuffix" | "filename_suffix" => Ok(GeneratedField::FilenameSuffix),
                            "maxDuration" | "max_duration" => Ok(GeneratedField::MaxDuration),
                            "maxBytes" | "max_bytes" => Ok(GeneratedField::MaxBytes),
                            "state" => Ok(GeneratedField::State),
                            "textConfig" | "text_config" => Ok(GeneratedField::TextConfig),
                            "avroConfig" | "avro_config" => Ok(GeneratedField::AvroConfig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CloudStorageConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.CloudStorageConfig")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<CloudStorageConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bucket__ = None;
                let mut filename_prefix__ = None;
                let mut filename_suffix__ = None;
                let mut max_duration__ = None;
                let mut max_bytes__ = None;
                let mut state__ = None;
                let mut output_format__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Bucket => {
                            if bucket__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bucket"));
                            }
                            bucket__ = Some(map.next_value()?);
                        }
                        GeneratedField::FilenamePrefix => {
                            if filename_prefix__.is_some() {
                                return Err(serde::de::Error::duplicate_field("filenamePrefix"));
                            }
                            filename_prefix__ = Some(map.next_value()?);
                        }
                        GeneratedField::FilenameSuffix => {
                            if filename_suffix__.is_some() {
                                return Err(serde::de::Error::duplicate_field("filenameSuffix"));
                            }
                            filename_suffix__ = Some(map.next_value()?);
                        }
                        GeneratedField::MaxDuration => {
                            if max_duration__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxDuration"));
                            }
                            max_duration__ = map.next_value()?;
                        }
                        GeneratedField::MaxBytes => {
                            if max_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxBytes"));
                            }
                            max_bytes__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::State => {
                            if state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state__ = Some(map.next_value::<cloud_storage_config::State>()? as i32);
                        }
                        GeneratedField::TextConfig => {
                            if output_format__.is_some() {
                                return Err(serde::de::Error::duplicate_field("textConfig"));
                            }
                            output_format__ = map.next_value::<::std::option::Option<_>>()?.map(cloud_storage_config::OutputFormat::TextConfig)
;
                        }
                        GeneratedField::AvroConfig => {
                            if output_format__.is_some() {
                                return Err(serde::de::Error::duplicate_field("avroConfig"));
                            }
                            output_format__ = map.next_value::<::std::option::Option<_>>()?.map(cloud_storage_config::OutputFormat::AvroConfig)
;
                        }
                    }
                }
                Ok(CloudStorageConfig {
                    bucket: bucket__.unwrap_or_default(),
                    filename_prefix: filename_prefix__.unwrap_or_default(),
                    filename_suffix: filename_suffix__.unwrap_or_default(),
                    max_duration: max_duration__,
                    max_bytes: max_bytes__.unwrap_or_default(),
                    state: state__.unwrap_or_default(),
                    output_format: output_format__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.CloudStorageConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for cloud_storage_config::AvroConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.write_metadata {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.CloudStorageConfig.AvroConfig", len)?;
        if self.write_metadata {
            struct_ser.serialize_field("writeMetadata", &self.write_metadata)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for cloud_storage_config::AvroConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "write_metadata",
            "writeMetadata",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            WriteMetadata,
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
                            "writeMetadata" | "write_metadata" => Ok(GeneratedField::WriteMetadata),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = cloud_storage_config::AvroConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.CloudStorageConfig.AvroConfig")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<cloud_storage_config::AvroConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut write_metadata__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::WriteMetadata => {
                            if write_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeMetadata"));
                            }
                            write_metadata__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(cloud_storage_config::AvroConfig {
                    write_metadata: write_metadata__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.CloudStorageConfig.AvroConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for cloud_storage_config::State {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "STATE_UNSPECIFIED",
            Self::Active => "ACTIVE",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::NotFound => "NOT_FOUND",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for cloud_storage_config::State {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "STATE_UNSPECIFIED",
            "ACTIVE",
            "PERMISSION_DENIED",
            "NOT_FOUND",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = cloud_storage_config::State;

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
                    .and_then(cloud_storage_config::State::from_i32)
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
                    .and_then(cloud_storage_config::State::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "STATE_UNSPECIFIED" => Ok(cloud_storage_config::State::Unspecified),
                    "ACTIVE" => Ok(cloud_storage_config::State::Active),
                    "PERMISSION_DENIED" => Ok(cloud_storage_config::State::PermissionDenied),
                    "NOT_FOUND" => Ok(cloud_storage_config::State::NotFound),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for cloud_storage_config::TextConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.pubsub.v1.CloudStorageConfig.TextConfig", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for cloud_storage_config::TextConfig {
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
            type Value = cloud_storage_config::TextConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.CloudStorageConfig.TextConfig")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<cloud_storage_config::TextConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(cloud_storage_config::TextConfig {
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.CloudStorageConfig.TextConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CommitSchemaRequest {
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
        if self.schema.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.CommitSchemaRequest", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if let Some(v) = self.schema.as_ref() {
            struct_ser.serialize_field("schema", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CommitSchemaRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "schema",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Schema,
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
                            "schema" => Ok(GeneratedField::Schema),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CommitSchemaRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.CommitSchemaRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<CommitSchemaRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut schema__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Schema => {
                            if schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schema"));
                            }
                            schema__ = map.next_value()?;
                        }
                    }
                }
                Ok(CommitSchemaRequest {
                    name: name__.unwrap_or_default(),
                    schema: schema__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.CommitSchemaRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CreateSchemaRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.parent.is_empty() {
            len += 1;
        }
        if self.schema.is_some() {
            len += 1;
        }
        if !self.schema_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.CreateSchemaRequest", len)?;
        if !self.parent.is_empty() {
            struct_ser.serialize_field("parent", &self.parent)?;
        }
        if let Some(v) = self.schema.as_ref() {
            struct_ser.serialize_field("schema", v)?;
        }
        if !self.schema_id.is_empty() {
            struct_ser.serialize_field("schemaId", &self.schema_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateSchemaRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "parent",
            "schema",
            "schema_id",
            "schemaId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Parent,
            Schema,
            SchemaId,
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
                            "parent" => Ok(GeneratedField::Parent),
                            "schema" => Ok(GeneratedField::Schema),
                            "schemaId" | "schema_id" => Ok(GeneratedField::SchemaId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CreateSchemaRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.CreateSchemaRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<CreateSchemaRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut parent__ = None;
                let mut schema__ = None;
                let mut schema_id__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Parent => {
                            if parent__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parent"));
                            }
                            parent__ = Some(map.next_value()?);
                        }
                        GeneratedField::Schema => {
                            if schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schema"));
                            }
                            schema__ = map.next_value()?;
                        }
                        GeneratedField::SchemaId => {
                            if schema_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schemaId"));
                            }
                            schema_id__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(CreateSchemaRequest {
                    parent: parent__.unwrap_or_default(),
                    schema: schema__,
                    schema_id: schema_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.CreateSchemaRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CreateSnapshotRequest {
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
        if !self.subscription.is_empty() {
            len += 1;
        }
        if !self.labels.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.CreateSnapshotRequest", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        if !self.labels.is_empty() {
            struct_ser.serialize_field("labels", &self.labels)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateSnapshotRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "subscription",
            "labels",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Subscription,
            Labels,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            "labels" => Ok(GeneratedField::Labels),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CreateSnapshotRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.CreateSnapshotRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<CreateSnapshotRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut subscription__ = None;
                let mut labels__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                        GeneratedField::Labels => {
                            if labels__.is_some() {
                                return Err(serde::de::Error::duplicate_field("labels"));
                            }
                            labels__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(CreateSnapshotRequest {
                    name: name__.unwrap_or_default(),
                    subscription: subscription__.unwrap_or_default(),
                    labels: labels__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.CreateSnapshotRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeadLetterPolicy {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.dead_letter_topic.is_empty() {
            len += 1;
        }
        if self.max_delivery_attempts != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.DeadLetterPolicy", len)?;
        if !self.dead_letter_topic.is_empty() {
            struct_ser.serialize_field("deadLetterTopic", &self.dead_letter_topic)?;
        }
        if self.max_delivery_attempts != 0 {
            struct_ser.serialize_field("maxDeliveryAttempts", &self.max_delivery_attempts)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeadLetterPolicy {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "dead_letter_topic",
            "deadLetterTopic",
            "max_delivery_attempts",
            "maxDeliveryAttempts",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DeadLetterTopic,
            MaxDeliveryAttempts,
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
                            "deadLetterTopic" | "dead_letter_topic" => Ok(GeneratedField::DeadLetterTopic),
                            "maxDeliveryAttempts" | "max_delivery_attempts" => Ok(GeneratedField::MaxDeliveryAttempts),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeadLetterPolicy;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.DeadLetterPolicy")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeadLetterPolicy, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut dead_letter_topic__ = None;
                let mut max_delivery_attempts__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::DeadLetterTopic => {
                            if dead_letter_topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deadLetterTopic"));
                            }
                            dead_letter_topic__ = Some(map.next_value()?);
                        }
                        GeneratedField::MaxDeliveryAttempts => {
                            if max_delivery_attempts__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxDeliveryAttempts"));
                            }
                            max_delivery_attempts__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(DeadLetterPolicy {
                    dead_letter_topic: dead_letter_topic__.unwrap_or_default(),
                    max_delivery_attempts: max_delivery_attempts__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.DeadLetterPolicy", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteSchemaRequest {
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
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.DeleteSchemaRequest", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteSchemaRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
            type Value = DeleteSchemaRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.DeleteSchemaRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteSchemaRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DeleteSchemaRequest {
                    name: name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.DeleteSchemaRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteSchemaRevisionRequest {
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
        if !self.revision_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.DeleteSchemaRevisionRequest", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.revision_id.is_empty() {
            struct_ser.serialize_field("revisionId", &self.revision_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteSchemaRevisionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "revision_id",
            "revisionId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            RevisionId,
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
                            "revisionId" | "revision_id" => Ok(GeneratedField::RevisionId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteSchemaRevisionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.DeleteSchemaRevisionRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteSchemaRevisionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut revision_id__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::RevisionId => {
                            if revision_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("revisionId"));
                            }
                            revision_id__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DeleteSchemaRevisionRequest {
                    name: name__.unwrap_or_default(),
                    revision_id: revision_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.DeleteSchemaRevisionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteSnapshotRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.snapshot.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.DeleteSnapshotRequest", len)?;
        if !self.snapshot.is_empty() {
            struct_ser.serialize_field("snapshot", &self.snapshot)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteSnapshotRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "snapshot",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Snapshot,
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
                            "snapshot" => Ok(GeneratedField::Snapshot),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteSnapshotRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.DeleteSnapshotRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteSnapshotRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut snapshot__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Snapshot => {
                            if snapshot__.is_some() {
                                return Err(serde::de::Error::duplicate_field("snapshot"));
                            }
                            snapshot__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DeleteSnapshotRequest {
                    snapshot: snapshot__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.DeleteSnapshotRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteSubscriptionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.DeleteSubscriptionRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteSubscriptionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteSubscriptionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.DeleteSubscriptionRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteSubscriptionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DeleteSubscriptionRequest {
                    subscription: subscription__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.DeleteSubscriptionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteTopicRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.topic.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.DeleteTopicRequest", len)?;
        if !self.topic.is_empty() {
            struct_ser.serialize_field("topic", &self.topic)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteTopicRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "topic",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Topic,
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
                            "topic" => Ok(GeneratedField::Topic),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteTopicRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.DeleteTopicRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DeleteTopicRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut topic__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Topic => {
                            if topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topic"));
                            }
                            topic__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DeleteTopicRequest {
                    topic: topic__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.DeleteTopicRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DetachSubscriptionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.DetachSubscriptionRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DetachSubscriptionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DetachSubscriptionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.DetachSubscriptionRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DetachSubscriptionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(DetachSubscriptionRequest {
                    subscription: subscription__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.DetachSubscriptionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DetachSubscriptionResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.pubsub.v1.DetachSubscriptionResponse", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DetachSubscriptionResponse {
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
            type Value = DetachSubscriptionResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.DetachSubscriptionResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<DetachSubscriptionResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(DetachSubscriptionResponse {
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.DetachSubscriptionResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Encoding {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "ENCODING_UNSPECIFIED",
            Self::Json => "JSON",
            Self::Binary => "BINARY",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for Encoding {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ENCODING_UNSPECIFIED",
            "JSON",
            "BINARY",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Encoding;

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
                    .and_then(Encoding::from_i32)
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
                    .and_then(Encoding::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "ENCODING_UNSPECIFIED" => Ok(Encoding::Unspecified),
                    "JSON" => Ok(Encoding::Json),
                    "BINARY" => Ok(Encoding::Binary),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ExpirationPolicy {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.ttl.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ExpirationPolicy", len)?;
        if let Some(v) = self.ttl.as_ref() {
            struct_ser.serialize_field("ttl", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExpirationPolicy {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ttl",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Ttl,
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
                            "ttl" => Ok(GeneratedField::Ttl),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExpirationPolicy;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ExpirationPolicy")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ExpirationPolicy, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ttl__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Ttl => {
                            if ttl__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ttl"));
                            }
                            ttl__ = map.next_value()?;
                        }
                    }
                }
                Ok(ExpirationPolicy {
                    ttl: ttl__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ExpirationPolicy", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetSchemaRequest {
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
        if self.view != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.GetSchemaRequest", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if self.view != 0 {
            let v = SchemaView::from_i32(self.view)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.view)))?;
            struct_ser.serialize_field("view", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetSchemaRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "view",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            View,
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
                            "view" => Ok(GeneratedField::View),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetSchemaRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.GetSchemaRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetSchemaRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut view__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::View => {
                            if view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("view"));
                            }
                            view__ = Some(map.next_value::<SchemaView>()? as i32);
                        }
                    }
                }
                Ok(GetSchemaRequest {
                    name: name__.unwrap_or_default(),
                    view: view__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.GetSchemaRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetSnapshotRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.snapshot.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.GetSnapshotRequest", len)?;
        if !self.snapshot.is_empty() {
            struct_ser.serialize_field("snapshot", &self.snapshot)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetSnapshotRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "snapshot",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Snapshot,
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
                            "snapshot" => Ok(GeneratedField::Snapshot),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetSnapshotRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.GetSnapshotRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetSnapshotRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut snapshot__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Snapshot => {
                            if snapshot__.is_some() {
                                return Err(serde::de::Error::duplicate_field("snapshot"));
                            }
                            snapshot__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(GetSnapshotRequest {
                    snapshot: snapshot__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.GetSnapshotRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetSubscriptionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.GetSubscriptionRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetSubscriptionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetSubscriptionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.GetSubscriptionRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetSubscriptionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(GetSubscriptionRequest {
                    subscription: subscription__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.GetSubscriptionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTopicRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.topic.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.GetTopicRequest", len)?;
        if !self.topic.is_empty() {
            struct_ser.serialize_field("topic", &self.topic)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTopicRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "topic",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Topic,
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
                            "topic" => Ok(GeneratedField::Topic),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTopicRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.GetTopicRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetTopicRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut topic__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Topic => {
                            if topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topic"));
                            }
                            topic__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(GetTopicRequest {
                    topic: topic__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.GetTopicRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListSchemaRevisionsRequest {
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
        if self.view != 0 {
            len += 1;
        }
        if self.page_size != 0 {
            len += 1;
        }
        if !self.page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListSchemaRevisionsRequest", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if self.view != 0 {
            let v = SchemaView::from_i32(self.view)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.view)))?;
            struct_ser.serialize_field("view", &v)?;
        }
        if self.page_size != 0 {
            struct_ser.serialize_field("pageSize", &self.page_size)?;
        }
        if !self.page_token.is_empty() {
            struct_ser.serialize_field("pageToken", &self.page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListSchemaRevisionsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "view",
            "page_size",
            "pageSize",
            "page_token",
            "pageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            View,
            PageSize,
            PageToken,
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
                            "view" => Ok(GeneratedField::View),
                            "pageSize" | "page_size" => Ok(GeneratedField::PageSize),
                            "pageToken" | "page_token" => Ok(GeneratedField::PageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListSchemaRevisionsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListSchemaRevisionsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListSchemaRevisionsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut view__ = None;
                let mut page_size__ = None;
                let mut page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::View => {
                            if view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("view"));
                            }
                            view__ = Some(map.next_value::<SchemaView>()? as i32);
                        }
                        GeneratedField::PageSize => {
                            if page_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageSize"));
                            }
                            page_size__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PageToken => {
                            if page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageToken"));
                            }
                            page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListSchemaRevisionsRequest {
                    name: name__.unwrap_or_default(),
                    view: view__.unwrap_or_default(),
                    page_size: page_size__.unwrap_or_default(),
                    page_token: page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListSchemaRevisionsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListSchemaRevisionsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.schemas.is_empty() {
            len += 1;
        }
        if !self.next_page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListSchemaRevisionsResponse", len)?;
        if !self.schemas.is_empty() {
            struct_ser.serialize_field("schemas", &self.schemas)?;
        }
        if !self.next_page_token.is_empty() {
            struct_ser.serialize_field("nextPageToken", &self.next_page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListSchemaRevisionsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "schemas",
            "next_page_token",
            "nextPageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Schemas,
            NextPageToken,
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
                            "schemas" => Ok(GeneratedField::Schemas),
                            "nextPageToken" | "next_page_token" => Ok(GeneratedField::NextPageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListSchemaRevisionsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListSchemaRevisionsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListSchemaRevisionsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut schemas__ = None;
                let mut next_page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Schemas => {
                            if schemas__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schemas"));
                            }
                            schemas__ = Some(map.next_value()?);
                        }
                        GeneratedField::NextPageToken => {
                            if next_page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextPageToken"));
                            }
                            next_page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListSchemaRevisionsResponse {
                    schemas: schemas__.unwrap_or_default(),
                    next_page_token: next_page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListSchemaRevisionsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListSchemasRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.parent.is_empty() {
            len += 1;
        }
        if self.view != 0 {
            len += 1;
        }
        if self.page_size != 0 {
            len += 1;
        }
        if !self.page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListSchemasRequest", len)?;
        if !self.parent.is_empty() {
            struct_ser.serialize_field("parent", &self.parent)?;
        }
        if self.view != 0 {
            let v = SchemaView::from_i32(self.view)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.view)))?;
            struct_ser.serialize_field("view", &v)?;
        }
        if self.page_size != 0 {
            struct_ser.serialize_field("pageSize", &self.page_size)?;
        }
        if !self.page_token.is_empty() {
            struct_ser.serialize_field("pageToken", &self.page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListSchemasRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "parent",
            "view",
            "page_size",
            "pageSize",
            "page_token",
            "pageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Parent,
            View,
            PageSize,
            PageToken,
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
                            "parent" => Ok(GeneratedField::Parent),
                            "view" => Ok(GeneratedField::View),
                            "pageSize" | "page_size" => Ok(GeneratedField::PageSize),
                            "pageToken" | "page_token" => Ok(GeneratedField::PageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListSchemasRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListSchemasRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListSchemasRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut parent__ = None;
                let mut view__ = None;
                let mut page_size__ = None;
                let mut page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Parent => {
                            if parent__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parent"));
                            }
                            parent__ = Some(map.next_value()?);
                        }
                        GeneratedField::View => {
                            if view__.is_some() {
                                return Err(serde::de::Error::duplicate_field("view"));
                            }
                            view__ = Some(map.next_value::<SchemaView>()? as i32);
                        }
                        GeneratedField::PageSize => {
                            if page_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageSize"));
                            }
                            page_size__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PageToken => {
                            if page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageToken"));
                            }
                            page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListSchemasRequest {
                    parent: parent__.unwrap_or_default(),
                    view: view__.unwrap_or_default(),
                    page_size: page_size__.unwrap_or_default(),
                    page_token: page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListSchemasRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListSchemasResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.schemas.is_empty() {
            len += 1;
        }
        if !self.next_page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListSchemasResponse", len)?;
        if !self.schemas.is_empty() {
            struct_ser.serialize_field("schemas", &self.schemas)?;
        }
        if !self.next_page_token.is_empty() {
            struct_ser.serialize_field("nextPageToken", &self.next_page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListSchemasResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "schemas",
            "next_page_token",
            "nextPageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Schemas,
            NextPageToken,
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
                            "schemas" => Ok(GeneratedField::Schemas),
                            "nextPageToken" | "next_page_token" => Ok(GeneratedField::NextPageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListSchemasResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListSchemasResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListSchemasResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut schemas__ = None;
                let mut next_page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Schemas => {
                            if schemas__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schemas"));
                            }
                            schemas__ = Some(map.next_value()?);
                        }
                        GeneratedField::NextPageToken => {
                            if next_page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextPageToken"));
                            }
                            next_page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListSchemasResponse {
                    schemas: schemas__.unwrap_or_default(),
                    next_page_token: next_page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListSchemasResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListSnapshotsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.project.is_empty() {
            len += 1;
        }
        if self.page_size != 0 {
            len += 1;
        }
        if !self.page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListSnapshotsRequest", len)?;
        if !self.project.is_empty() {
            struct_ser.serialize_field("project", &self.project)?;
        }
        if self.page_size != 0 {
            struct_ser.serialize_field("pageSize", &self.page_size)?;
        }
        if !self.page_token.is_empty() {
            struct_ser.serialize_field("pageToken", &self.page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListSnapshotsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "project",
            "page_size",
            "pageSize",
            "page_token",
            "pageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Project,
            PageSize,
            PageToken,
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
                            "project" => Ok(GeneratedField::Project),
                            "pageSize" | "page_size" => Ok(GeneratedField::PageSize),
                            "pageToken" | "page_token" => Ok(GeneratedField::PageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListSnapshotsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListSnapshotsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListSnapshotsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut project__ = None;
                let mut page_size__ = None;
                let mut page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Project => {
                            if project__.is_some() {
                                return Err(serde::de::Error::duplicate_field("project"));
                            }
                            project__ = Some(map.next_value()?);
                        }
                        GeneratedField::PageSize => {
                            if page_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageSize"));
                            }
                            page_size__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PageToken => {
                            if page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageToken"));
                            }
                            page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListSnapshotsRequest {
                    project: project__.unwrap_or_default(),
                    page_size: page_size__.unwrap_or_default(),
                    page_token: page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListSnapshotsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListSnapshotsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.snapshots.is_empty() {
            len += 1;
        }
        if !self.next_page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListSnapshotsResponse", len)?;
        if !self.snapshots.is_empty() {
            struct_ser.serialize_field("snapshots", &self.snapshots)?;
        }
        if !self.next_page_token.is_empty() {
            struct_ser.serialize_field("nextPageToken", &self.next_page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListSnapshotsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "snapshots",
            "next_page_token",
            "nextPageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Snapshots,
            NextPageToken,
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
                            "snapshots" => Ok(GeneratedField::Snapshots),
                            "nextPageToken" | "next_page_token" => Ok(GeneratedField::NextPageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListSnapshotsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListSnapshotsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListSnapshotsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut snapshots__ = None;
                let mut next_page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Snapshots => {
                            if snapshots__.is_some() {
                                return Err(serde::de::Error::duplicate_field("snapshots"));
                            }
                            snapshots__ = Some(map.next_value()?);
                        }
                        GeneratedField::NextPageToken => {
                            if next_page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextPageToken"));
                            }
                            next_page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListSnapshotsResponse {
                    snapshots: snapshots__.unwrap_or_default(),
                    next_page_token: next_page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListSnapshotsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListSubscriptionsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.project.is_empty() {
            len += 1;
        }
        if self.page_size != 0 {
            len += 1;
        }
        if !self.page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListSubscriptionsRequest", len)?;
        if !self.project.is_empty() {
            struct_ser.serialize_field("project", &self.project)?;
        }
        if self.page_size != 0 {
            struct_ser.serialize_field("pageSize", &self.page_size)?;
        }
        if !self.page_token.is_empty() {
            struct_ser.serialize_field("pageToken", &self.page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListSubscriptionsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "project",
            "page_size",
            "pageSize",
            "page_token",
            "pageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Project,
            PageSize,
            PageToken,
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
                            "project" => Ok(GeneratedField::Project),
                            "pageSize" | "page_size" => Ok(GeneratedField::PageSize),
                            "pageToken" | "page_token" => Ok(GeneratedField::PageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListSubscriptionsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListSubscriptionsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListSubscriptionsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut project__ = None;
                let mut page_size__ = None;
                let mut page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Project => {
                            if project__.is_some() {
                                return Err(serde::de::Error::duplicate_field("project"));
                            }
                            project__ = Some(map.next_value()?);
                        }
                        GeneratedField::PageSize => {
                            if page_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageSize"));
                            }
                            page_size__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PageToken => {
                            if page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageToken"));
                            }
                            page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListSubscriptionsRequest {
                    project: project__.unwrap_or_default(),
                    page_size: page_size__.unwrap_or_default(),
                    page_token: page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListSubscriptionsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListSubscriptionsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscriptions.is_empty() {
            len += 1;
        }
        if !self.next_page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListSubscriptionsResponse", len)?;
        if !self.subscriptions.is_empty() {
            struct_ser.serialize_field("subscriptions", &self.subscriptions)?;
        }
        if !self.next_page_token.is_empty() {
            struct_ser.serialize_field("nextPageToken", &self.next_page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListSubscriptionsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscriptions",
            "next_page_token",
            "nextPageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscriptions,
            NextPageToken,
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
                            "subscriptions" => Ok(GeneratedField::Subscriptions),
                            "nextPageToken" | "next_page_token" => Ok(GeneratedField::NextPageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListSubscriptionsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListSubscriptionsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListSubscriptionsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscriptions__ = None;
                let mut next_page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscriptions => {
                            if subscriptions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscriptions"));
                            }
                            subscriptions__ = Some(map.next_value()?);
                        }
                        GeneratedField::NextPageToken => {
                            if next_page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextPageToken"));
                            }
                            next_page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListSubscriptionsResponse {
                    subscriptions: subscriptions__.unwrap_or_default(),
                    next_page_token: next_page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListSubscriptionsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListTopicSnapshotsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.topic.is_empty() {
            len += 1;
        }
        if self.page_size != 0 {
            len += 1;
        }
        if !self.page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListTopicSnapshotsRequest", len)?;
        if !self.topic.is_empty() {
            struct_ser.serialize_field("topic", &self.topic)?;
        }
        if self.page_size != 0 {
            struct_ser.serialize_field("pageSize", &self.page_size)?;
        }
        if !self.page_token.is_empty() {
            struct_ser.serialize_field("pageToken", &self.page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListTopicSnapshotsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "topic",
            "page_size",
            "pageSize",
            "page_token",
            "pageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Topic,
            PageSize,
            PageToken,
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
                            "topic" => Ok(GeneratedField::Topic),
                            "pageSize" | "page_size" => Ok(GeneratedField::PageSize),
                            "pageToken" | "page_token" => Ok(GeneratedField::PageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListTopicSnapshotsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListTopicSnapshotsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListTopicSnapshotsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut topic__ = None;
                let mut page_size__ = None;
                let mut page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Topic => {
                            if topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topic"));
                            }
                            topic__ = Some(map.next_value()?);
                        }
                        GeneratedField::PageSize => {
                            if page_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageSize"));
                            }
                            page_size__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PageToken => {
                            if page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageToken"));
                            }
                            page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListTopicSnapshotsRequest {
                    topic: topic__.unwrap_or_default(),
                    page_size: page_size__.unwrap_or_default(),
                    page_token: page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListTopicSnapshotsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListTopicSnapshotsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.snapshots.is_empty() {
            len += 1;
        }
        if !self.next_page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListTopicSnapshotsResponse", len)?;
        if !self.snapshots.is_empty() {
            struct_ser.serialize_field("snapshots", &self.snapshots)?;
        }
        if !self.next_page_token.is_empty() {
            struct_ser.serialize_field("nextPageToken", &self.next_page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListTopicSnapshotsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "snapshots",
            "next_page_token",
            "nextPageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Snapshots,
            NextPageToken,
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
                            "snapshots" => Ok(GeneratedField::Snapshots),
                            "nextPageToken" | "next_page_token" => Ok(GeneratedField::NextPageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListTopicSnapshotsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListTopicSnapshotsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListTopicSnapshotsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut snapshots__ = None;
                let mut next_page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Snapshots => {
                            if snapshots__.is_some() {
                                return Err(serde::de::Error::duplicate_field("snapshots"));
                            }
                            snapshots__ = Some(map.next_value()?);
                        }
                        GeneratedField::NextPageToken => {
                            if next_page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextPageToken"));
                            }
                            next_page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListTopicSnapshotsResponse {
                    snapshots: snapshots__.unwrap_or_default(),
                    next_page_token: next_page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListTopicSnapshotsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListTopicSubscriptionsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.topic.is_empty() {
            len += 1;
        }
        if self.page_size != 0 {
            len += 1;
        }
        if !self.page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListTopicSubscriptionsRequest", len)?;
        if !self.topic.is_empty() {
            struct_ser.serialize_field("topic", &self.topic)?;
        }
        if self.page_size != 0 {
            struct_ser.serialize_field("pageSize", &self.page_size)?;
        }
        if !self.page_token.is_empty() {
            struct_ser.serialize_field("pageToken", &self.page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListTopicSubscriptionsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "topic",
            "page_size",
            "pageSize",
            "page_token",
            "pageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Topic,
            PageSize,
            PageToken,
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
                            "topic" => Ok(GeneratedField::Topic),
                            "pageSize" | "page_size" => Ok(GeneratedField::PageSize),
                            "pageToken" | "page_token" => Ok(GeneratedField::PageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListTopicSubscriptionsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListTopicSubscriptionsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListTopicSubscriptionsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut topic__ = None;
                let mut page_size__ = None;
                let mut page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Topic => {
                            if topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topic"));
                            }
                            topic__ = Some(map.next_value()?);
                        }
                        GeneratedField::PageSize => {
                            if page_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageSize"));
                            }
                            page_size__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PageToken => {
                            if page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageToken"));
                            }
                            page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListTopicSubscriptionsRequest {
                    topic: topic__.unwrap_or_default(),
                    page_size: page_size__.unwrap_or_default(),
                    page_token: page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListTopicSubscriptionsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListTopicSubscriptionsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscriptions.is_empty() {
            len += 1;
        }
        if !self.next_page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListTopicSubscriptionsResponse", len)?;
        if !self.subscriptions.is_empty() {
            struct_ser.serialize_field("subscriptions", &self.subscriptions)?;
        }
        if !self.next_page_token.is_empty() {
            struct_ser.serialize_field("nextPageToken", &self.next_page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListTopicSubscriptionsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscriptions",
            "next_page_token",
            "nextPageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscriptions,
            NextPageToken,
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
                            "subscriptions" => Ok(GeneratedField::Subscriptions),
                            "nextPageToken" | "next_page_token" => Ok(GeneratedField::NextPageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListTopicSubscriptionsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListTopicSubscriptionsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListTopicSubscriptionsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscriptions__ = None;
                let mut next_page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscriptions => {
                            if subscriptions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscriptions"));
                            }
                            subscriptions__ = Some(map.next_value()?);
                        }
                        GeneratedField::NextPageToken => {
                            if next_page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextPageToken"));
                            }
                            next_page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListTopicSubscriptionsResponse {
                    subscriptions: subscriptions__.unwrap_or_default(),
                    next_page_token: next_page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListTopicSubscriptionsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListTopicsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.project.is_empty() {
            len += 1;
        }
        if self.page_size != 0 {
            len += 1;
        }
        if !self.page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListTopicsRequest", len)?;
        if !self.project.is_empty() {
            struct_ser.serialize_field("project", &self.project)?;
        }
        if self.page_size != 0 {
            struct_ser.serialize_field("pageSize", &self.page_size)?;
        }
        if !self.page_token.is_empty() {
            struct_ser.serialize_field("pageToken", &self.page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListTopicsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "project",
            "page_size",
            "pageSize",
            "page_token",
            "pageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Project,
            PageSize,
            PageToken,
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
                            "project" => Ok(GeneratedField::Project),
                            "pageSize" | "page_size" => Ok(GeneratedField::PageSize),
                            "pageToken" | "page_token" => Ok(GeneratedField::PageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListTopicsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListTopicsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListTopicsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut project__ = None;
                let mut page_size__ = None;
                let mut page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Project => {
                            if project__.is_some() {
                                return Err(serde::de::Error::duplicate_field("project"));
                            }
                            project__ = Some(map.next_value()?);
                        }
                        GeneratedField::PageSize => {
                            if page_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageSize"));
                            }
                            page_size__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PageToken => {
                            if page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pageToken"));
                            }
                            page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListTopicsRequest {
                    project: project__.unwrap_or_default(),
                    page_size: page_size__.unwrap_or_default(),
                    page_token: page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListTopicsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ListTopicsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.topics.is_empty() {
            len += 1;
        }
        if !self.next_page_token.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ListTopicsResponse", len)?;
        if !self.topics.is_empty() {
            struct_ser.serialize_field("topics", &self.topics)?;
        }
        if !self.next_page_token.is_empty() {
            struct_ser.serialize_field("nextPageToken", &self.next_page_token)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ListTopicsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "topics",
            "next_page_token",
            "nextPageToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Topics,
            NextPageToken,
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
                            "topics" => Ok(GeneratedField::Topics),
                            "nextPageToken" | "next_page_token" => Ok(GeneratedField::NextPageToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ListTopicsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ListTopicsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ListTopicsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut topics__ = None;
                let mut next_page_token__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Topics => {
                            if topics__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topics"));
                            }
                            topics__ = Some(map.next_value()?);
                        }
                        GeneratedField::NextPageToken => {
                            if next_page_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextPageToken"));
                            }
                            next_page_token__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(ListTopicsResponse {
                    topics: topics__.unwrap_or_default(),
                    next_page_token: next_page_token__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ListTopicsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MessageStoragePolicy {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.allowed_persistence_regions.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.MessageStoragePolicy", len)?;
        if !self.allowed_persistence_regions.is_empty() {
            struct_ser.serialize_field("allowedPersistenceRegions", &self.allowed_persistence_regions)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MessageStoragePolicy {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "allowed_persistence_regions",
            "allowedPersistenceRegions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AllowedPersistenceRegions,
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
                            "allowedPersistenceRegions" | "allowed_persistence_regions" => Ok(GeneratedField::AllowedPersistenceRegions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MessageStoragePolicy;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.MessageStoragePolicy")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MessageStoragePolicy, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut allowed_persistence_regions__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::AllowedPersistenceRegions => {
                            if allowed_persistence_regions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("allowedPersistenceRegions"));
                            }
                            allowed_persistence_regions__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(MessageStoragePolicy {
                    allowed_persistence_regions: allowed_persistence_regions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.MessageStoragePolicy", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ModifyAckDeadlineRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        if !self.ack_ids.is_empty() {
            len += 1;
        }
        if self.ack_deadline_seconds != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ModifyAckDeadlineRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        if !self.ack_ids.is_empty() {
            struct_ser.serialize_field("ackIds", &self.ack_ids)?;
        }
        if self.ack_deadline_seconds != 0 {
            struct_ser.serialize_field("ackDeadlineSeconds", &self.ack_deadline_seconds)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ModifyAckDeadlineRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
            "ack_ids",
            "ackIds",
            "ack_deadline_seconds",
            "ackDeadlineSeconds",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
            AckIds,
            AckDeadlineSeconds,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            "ackIds" | "ack_ids" => Ok(GeneratedField::AckIds),
                            "ackDeadlineSeconds" | "ack_deadline_seconds" => Ok(GeneratedField::AckDeadlineSeconds),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ModifyAckDeadlineRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ModifyAckDeadlineRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ModifyAckDeadlineRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                let mut ack_ids__ = None;
                let mut ack_deadline_seconds__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                        GeneratedField::AckIds => {
                            if ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ackIds"));
                            }
                            ack_ids__ = Some(map.next_value()?);
                        }
                        GeneratedField::AckDeadlineSeconds => {
                            if ack_deadline_seconds__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ackDeadlineSeconds"));
                            }
                            ack_deadline_seconds__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(ModifyAckDeadlineRequest {
                    subscription: subscription__.unwrap_or_default(),
                    ack_ids: ack_ids__.unwrap_or_default(),
                    ack_deadline_seconds: ack_deadline_seconds__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ModifyAckDeadlineRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ModifyPushConfigRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        if self.push_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ModifyPushConfigRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        if let Some(v) = self.push_config.as_ref() {
            struct_ser.serialize_field("pushConfig", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ModifyPushConfigRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
            "push_config",
            "pushConfig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
            PushConfig,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            "pushConfig" | "push_config" => Ok(GeneratedField::PushConfig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ModifyPushConfigRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ModifyPushConfigRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ModifyPushConfigRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                let mut push_config__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                        GeneratedField::PushConfig => {
                            if push_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pushConfig"));
                            }
                            push_config__ = map.next_value()?;
                        }
                    }
                }
                Ok(ModifyPushConfigRequest {
                    subscription: subscription__.unwrap_or_default(),
                    push_config: push_config__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ModifyPushConfigRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PublishRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.topic.is_empty() {
            len += 1;
        }
        if !self.messages.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.PublishRequest", len)?;
        if !self.topic.is_empty() {
            struct_ser.serialize_field("topic", &self.topic)?;
        }
        if !self.messages.is_empty() {
            struct_ser.serialize_field("messages", &self.messages)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PublishRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "topic",
            "messages",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Topic,
            Messages,
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
                            "topic" => Ok(GeneratedField::Topic),
                            "messages" => Ok(GeneratedField::Messages),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PublishRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PublishRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PublishRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut topic__ = None;
                let mut messages__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Topic => {
                            if topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topic"));
                            }
                            topic__ = Some(map.next_value()?);
                        }
                        GeneratedField::Messages => {
                            if messages__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messages"));
                            }
                            messages__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(PublishRequest {
                    topic: topic__.unwrap_or_default(),
                    messages: messages__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PublishRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PublishResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.message_ids.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.PublishResponse", len)?;
        if !self.message_ids.is_empty() {
            struct_ser.serialize_field("messageIds", &self.message_ids)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PublishResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "message_ids",
            "messageIds",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MessageIds,
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
                            "messageIds" | "message_ids" => Ok(GeneratedField::MessageIds),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PublishResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PublishResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PublishResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut message_ids__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::MessageIds => {
                            if message_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messageIds"));
                            }
                            message_ids__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(PublishResponse {
                    message_ids: message_ids__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PublishResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PubsubMessage {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.data.is_empty() {
            len += 1;
        }
        if !self.attributes.is_empty() {
            len += 1;
        }
        if !self.message_id.is_empty() {
            len += 1;
        }
        if self.publish_time.is_some() {
            len += 1;
        }
        if !self.ordering_key.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.PubsubMessage", len)?;
        if !self.data.is_empty() {
            struct_ser.serialize_field("data", pbjson::private::base64::encode(&self.data).as_str())?;
        }
        if !self.attributes.is_empty() {
            struct_ser.serialize_field("attributes", &self.attributes)?;
        }
        if !self.message_id.is_empty() {
            struct_ser.serialize_field("messageId", &self.message_id)?;
        }
        if let Some(v) = self.publish_time.as_ref() {
            struct_ser.serialize_field("publishTime", v)?;
        }
        if !self.ordering_key.is_empty() {
            struct_ser.serialize_field("orderingKey", &self.ordering_key)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PubsubMessage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "data",
            "attributes",
            "message_id",
            "messageId",
            "publish_time",
            "publishTime",
            "ordering_key",
            "orderingKey",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Data,
            Attributes,
            MessageId,
            PublishTime,
            OrderingKey,
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
                            "data" => Ok(GeneratedField::Data),
                            "attributes" => Ok(GeneratedField::Attributes),
                            "messageId" | "message_id" => Ok(GeneratedField::MessageId),
                            "publishTime" | "publish_time" => Ok(GeneratedField::PublishTime),
                            "orderingKey" | "ordering_key" => Ok(GeneratedField::OrderingKey),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PubsubMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PubsubMessage")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PubsubMessage, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data__ = None;
                let mut attributes__ = None;
                let mut message_id__ = None;
                let mut publish_time__ = None;
                let mut ordering_key__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Attributes => {
                            if attributes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("attributes"));
                            }
                            attributes__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::MessageId => {
                            if message_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messageId"));
                            }
                            message_id__ = Some(map.next_value()?);
                        }
                        GeneratedField::PublishTime => {
                            if publish_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publishTime"));
                            }
                            publish_time__ = map.next_value()?;
                        }
                        GeneratedField::OrderingKey => {
                            if ordering_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("orderingKey"));
                            }
                            ordering_key__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(PubsubMessage {
                    data: data__.unwrap_or_default(),
                    attributes: attributes__.unwrap_or_default(),
                    message_id: message_id__.unwrap_or_default(),
                    publish_time: publish_time__,
                    ordering_key: ordering_key__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PubsubMessage", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PullRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        if self.return_immediately {
            len += 1;
        }
        if self.max_messages != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.PullRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        if self.return_immediately {
            struct_ser.serialize_field("returnImmediately", &self.return_immediately)?;
        }
        if self.max_messages != 0 {
            struct_ser.serialize_field("maxMessages", &self.max_messages)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PullRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
            "return_immediately",
            "returnImmediately",
            "max_messages",
            "maxMessages",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
            ReturnImmediately,
            MaxMessages,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            "returnImmediately" | "return_immediately" => Ok(GeneratedField::ReturnImmediately),
                            "maxMessages" | "max_messages" => Ok(GeneratedField::MaxMessages),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PullRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PullRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PullRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                let mut return_immediately__ = None;
                let mut max_messages__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                        GeneratedField::ReturnImmediately => {
                            if return_immediately__.is_some() {
                                return Err(serde::de::Error::duplicate_field("returnImmediately"));
                            }
                            return_immediately__ = Some(map.next_value()?);
                        }
                        GeneratedField::MaxMessages => {
                            if max_messages__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxMessages"));
                            }
                            max_messages__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(PullRequest {
                    subscription: subscription__.unwrap_or_default(),
                    return_immediately: return_immediately__.unwrap_or_default(),
                    max_messages: max_messages__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PullRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PullResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.received_messages.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.PullResponse", len)?;
        if !self.received_messages.is_empty() {
            struct_ser.serialize_field("receivedMessages", &self.received_messages)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PullResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "received_messages",
            "receivedMessages",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ReceivedMessages,
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
                            "receivedMessages" | "received_messages" => Ok(GeneratedField::ReceivedMessages),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PullResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PullResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PullResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut received_messages__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ReceivedMessages => {
                            if received_messages__.is_some() {
                                return Err(serde::de::Error::duplicate_field("receivedMessages"));
                            }
                            received_messages__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(PullResponse {
                    received_messages: received_messages__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PullResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PushConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.push_endpoint.is_empty() {
            len += 1;
        }
        if !self.attributes.is_empty() {
            len += 1;
        }
        if self.authentication_method.is_some() {
            len += 1;
        }
        if self.wrapper.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.PushConfig", len)?;
        if !self.push_endpoint.is_empty() {
            struct_ser.serialize_field("pushEndpoint", &self.push_endpoint)?;
        }
        if !self.attributes.is_empty() {
            struct_ser.serialize_field("attributes", &self.attributes)?;
        }
        if let Some(v) = self.authentication_method.as_ref() {
            match v {
                push_config::AuthenticationMethod::OidcToken(v) => {
                    struct_ser.serialize_field("oidcToken", v)?;
                }
            }
        }
        if let Some(v) = self.wrapper.as_ref() {
            match v {
                push_config::Wrapper::PubsubWrapper(v) => {
                    struct_ser.serialize_field("pubsubWrapper", v)?;
                }
                push_config::Wrapper::NoWrapper(v) => {
                    struct_ser.serialize_field("noWrapper", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PushConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "push_endpoint",
            "pushEndpoint",
            "attributes",
            "oidc_token",
            "oidcToken",
            "pubsub_wrapper",
            "pubsubWrapper",
            "no_wrapper",
            "noWrapper",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PushEndpoint,
            Attributes,
            OidcToken,
            PubsubWrapper,
            NoWrapper,
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
                            "pushEndpoint" | "push_endpoint" => Ok(GeneratedField::PushEndpoint),
                            "attributes" => Ok(GeneratedField::Attributes),
                            "oidcToken" | "oidc_token" => Ok(GeneratedField::OidcToken),
                            "pubsubWrapper" | "pubsub_wrapper" => Ok(GeneratedField::PubsubWrapper),
                            "noWrapper" | "no_wrapper" => Ok(GeneratedField::NoWrapper),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PushConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PushConfig")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PushConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut push_endpoint__ = None;
                let mut attributes__ = None;
                let mut authentication_method__ = None;
                let mut wrapper__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::PushEndpoint => {
                            if push_endpoint__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pushEndpoint"));
                            }
                            push_endpoint__ = Some(map.next_value()?);
                        }
                        GeneratedField::Attributes => {
                            if attributes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("attributes"));
                            }
                            attributes__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::OidcToken => {
                            if authentication_method__.is_some() {
                                return Err(serde::de::Error::duplicate_field("oidcToken"));
                            }
                            authentication_method__ = map.next_value::<::std::option::Option<_>>()?.map(push_config::AuthenticationMethod::OidcToken)
;
                        }
                        GeneratedField::PubsubWrapper => {
                            if wrapper__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pubsubWrapper"));
                            }
                            wrapper__ = map.next_value::<::std::option::Option<_>>()?.map(push_config::Wrapper::PubsubWrapper)
;
                        }
                        GeneratedField::NoWrapper => {
                            if wrapper__.is_some() {
                                return Err(serde::de::Error::duplicate_field("noWrapper"));
                            }
                            wrapper__ = map.next_value::<::std::option::Option<_>>()?.map(push_config::Wrapper::NoWrapper)
;
                        }
                    }
                }
                Ok(PushConfig {
                    push_endpoint: push_endpoint__.unwrap_or_default(),
                    attributes: attributes__.unwrap_or_default(),
                    authentication_method: authentication_method__,
                    wrapper: wrapper__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PushConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for push_config::NoWrapper {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.write_metadata {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.PushConfig.NoWrapper", len)?;
        if self.write_metadata {
            struct_ser.serialize_field("writeMetadata", &self.write_metadata)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for push_config::NoWrapper {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "write_metadata",
            "writeMetadata",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            WriteMetadata,
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
                            "writeMetadata" | "write_metadata" => Ok(GeneratedField::WriteMetadata),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = push_config::NoWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PushConfig.NoWrapper")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<push_config::NoWrapper, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut write_metadata__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::WriteMetadata => {
                            if write_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("writeMetadata"));
                            }
                            write_metadata__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(push_config::NoWrapper {
                    write_metadata: write_metadata__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PushConfig.NoWrapper", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for push_config::OidcToken {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.service_account_email.is_empty() {
            len += 1;
        }
        if !self.audience.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.PushConfig.OidcToken", len)?;
        if !self.service_account_email.is_empty() {
            struct_ser.serialize_field("serviceAccountEmail", &self.service_account_email)?;
        }
        if !self.audience.is_empty() {
            struct_ser.serialize_field("audience", &self.audience)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for push_config::OidcToken {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "service_account_email",
            "serviceAccountEmail",
            "audience",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ServiceAccountEmail,
            Audience,
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
                            "serviceAccountEmail" | "service_account_email" => Ok(GeneratedField::ServiceAccountEmail),
                            "audience" => Ok(GeneratedField::Audience),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = push_config::OidcToken;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PushConfig.OidcToken")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<push_config::OidcToken, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut service_account_email__ = None;
                let mut audience__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ServiceAccountEmail => {
                            if service_account_email__.is_some() {
                                return Err(serde::de::Error::duplicate_field("serviceAccountEmail"));
                            }
                            service_account_email__ = Some(map.next_value()?);
                        }
                        GeneratedField::Audience => {
                            if audience__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audience"));
                            }
                            audience__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(push_config::OidcToken {
                    service_account_email: service_account_email__.unwrap_or_default(),
                    audience: audience__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PushConfig.OidcToken", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for push_config::PubsubWrapper {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.pubsub.v1.PushConfig.PubsubWrapper", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for push_config::PubsubWrapper {
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
            type Value = push_config::PubsubWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.PushConfig.PubsubWrapper")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<push_config::PubsubWrapper, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(push_config::PubsubWrapper {
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.PushConfig.PubsubWrapper", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ReceivedMessage {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.ack_id.is_empty() {
            len += 1;
        }
        if self.message.is_some() {
            len += 1;
        }
        if self.delivery_attempt != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ReceivedMessage", len)?;
        if !self.ack_id.is_empty() {
            struct_ser.serialize_field("ackId", &self.ack_id)?;
        }
        if let Some(v) = self.message.as_ref() {
            struct_ser.serialize_field("message", v)?;
        }
        if self.delivery_attempt != 0 {
            struct_ser.serialize_field("deliveryAttempt", &self.delivery_attempt)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ReceivedMessage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ack_id",
            "ackId",
            "message",
            "delivery_attempt",
            "deliveryAttempt",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AckId,
            Message,
            DeliveryAttempt,
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
                            "ackId" | "ack_id" => Ok(GeneratedField::AckId),
                            "message" => Ok(GeneratedField::Message),
                            "deliveryAttempt" | "delivery_attempt" => Ok(GeneratedField::DeliveryAttempt),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ReceivedMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ReceivedMessage")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ReceivedMessage, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ack_id__ = None;
                let mut message__ = None;
                let mut delivery_attempt__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::AckId => {
                            if ack_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ackId"));
                            }
                            ack_id__ = Some(map.next_value()?);
                        }
                        GeneratedField::Message => {
                            if message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message__ = map.next_value()?;
                        }
                        GeneratedField::DeliveryAttempt => {
                            if delivery_attempt__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deliveryAttempt"));
                            }
                            delivery_attempt__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(ReceivedMessage {
                    ack_id: ack_id__.unwrap_or_default(),
                    message: message__,
                    delivery_attempt: delivery_attempt__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ReceivedMessage", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RetryPolicy {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.minimum_backoff.is_some() {
            len += 1;
        }
        if self.maximum_backoff.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.RetryPolicy", len)?;
        if let Some(v) = self.minimum_backoff.as_ref() {
            struct_ser.serialize_field("minimumBackoff", v)?;
        }
        if let Some(v) = self.maximum_backoff.as_ref() {
            struct_ser.serialize_field("maximumBackoff", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RetryPolicy {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "minimum_backoff",
            "minimumBackoff",
            "maximum_backoff",
            "maximumBackoff",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MinimumBackoff,
            MaximumBackoff,
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
                            "minimumBackoff" | "minimum_backoff" => Ok(GeneratedField::MinimumBackoff),
                            "maximumBackoff" | "maximum_backoff" => Ok(GeneratedField::MaximumBackoff),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RetryPolicy;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.RetryPolicy")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<RetryPolicy, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut minimum_backoff__ = None;
                let mut maximum_backoff__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::MinimumBackoff => {
                            if minimum_backoff__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minimumBackoff"));
                            }
                            minimum_backoff__ = map.next_value()?;
                        }
                        GeneratedField::MaximumBackoff => {
                            if maximum_backoff__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maximumBackoff"));
                            }
                            maximum_backoff__ = map.next_value()?;
                        }
                    }
                }
                Ok(RetryPolicy {
                    minimum_backoff: minimum_backoff__,
                    maximum_backoff: maximum_backoff__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.RetryPolicy", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RollbackSchemaRequest {
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
        if !self.revision_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.RollbackSchemaRequest", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.revision_id.is_empty() {
            struct_ser.serialize_field("revisionId", &self.revision_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RollbackSchemaRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "revision_id",
            "revisionId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            RevisionId,
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
                            "revisionId" | "revision_id" => Ok(GeneratedField::RevisionId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RollbackSchemaRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.RollbackSchemaRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<RollbackSchemaRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut revision_id__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::RevisionId => {
                            if revision_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("revisionId"));
                            }
                            revision_id__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(RollbackSchemaRequest {
                    name: name__.unwrap_or_default(),
                    revision_id: revision_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.RollbackSchemaRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Schema {
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
        if self.r#type != 0 {
            len += 1;
        }
        if !self.definition.is_empty() {
            len += 1;
        }
        if !self.revision_id.is_empty() {
            len += 1;
        }
        if self.revision_create_time.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.Schema", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if self.r#type != 0 {
            let v = schema::Type::from_i32(self.r#type)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if !self.definition.is_empty() {
            struct_ser.serialize_field("definition", &self.definition)?;
        }
        if !self.revision_id.is_empty() {
            struct_ser.serialize_field("revisionId", &self.revision_id)?;
        }
        if let Some(v) = self.revision_create_time.as_ref() {
            struct_ser.serialize_field("revisionCreateTime", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Schema {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "type",
            "definition",
            "revision_id",
            "revisionId",
            "revision_create_time",
            "revisionCreateTime",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Type,
            Definition,
            RevisionId,
            RevisionCreateTime,
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
                            "definition" => Ok(GeneratedField::Definition),
                            "revisionId" | "revision_id" => Ok(GeneratedField::RevisionId),
                            "revisionCreateTime" | "revision_create_time" => Ok(GeneratedField::RevisionCreateTime),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Schema;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.Schema")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Schema, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut r#type__ = None;
                let mut definition__ = None;
                let mut revision_id__ = None;
                let mut revision_create_time__ = None;
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
                            r#type__ = Some(map.next_value::<schema::Type>()? as i32);
                        }
                        GeneratedField::Definition => {
                            if definition__.is_some() {
                                return Err(serde::de::Error::duplicate_field("definition"));
                            }
                            definition__ = Some(map.next_value()?);
                        }
                        GeneratedField::RevisionId => {
                            if revision_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("revisionId"));
                            }
                            revision_id__ = Some(map.next_value()?);
                        }
                        GeneratedField::RevisionCreateTime => {
                            if revision_create_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("revisionCreateTime"));
                            }
                            revision_create_time__ = map.next_value()?;
                        }
                    }
                }
                Ok(Schema {
                    name: name__.unwrap_or_default(),
                    r#type: r#type__.unwrap_or_default(),
                    definition: definition__.unwrap_or_default(),
                    revision_id: revision_id__.unwrap_or_default(),
                    revision_create_time: revision_create_time__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.Schema", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for schema::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::ProtocolBuffer => "PROTOCOL_BUFFER",
            Self::Avro => "AVRO",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for schema::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "PROTOCOL_BUFFER",
            "AVRO",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = schema::Type;

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
                    .and_then(schema::Type::from_i32)
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
                    .and_then(schema::Type::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(schema::Type::Unspecified),
                    "PROTOCOL_BUFFER" => Ok(schema::Type::ProtocolBuffer),
                    "AVRO" => Ok(schema::Type::Avro),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for SchemaSettings {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.schema.is_empty() {
            len += 1;
        }
        if self.encoding != 0 {
            len += 1;
        }
        if !self.first_revision_id.is_empty() {
            len += 1;
        }
        if !self.last_revision_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.SchemaSettings", len)?;
        if !self.schema.is_empty() {
            struct_ser.serialize_field("schema", &self.schema)?;
        }
        if self.encoding != 0 {
            let v = Encoding::from_i32(self.encoding)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.encoding)))?;
            struct_ser.serialize_field("encoding", &v)?;
        }
        if !self.first_revision_id.is_empty() {
            struct_ser.serialize_field("firstRevisionId", &self.first_revision_id)?;
        }
        if !self.last_revision_id.is_empty() {
            struct_ser.serialize_field("lastRevisionId", &self.last_revision_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SchemaSettings {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "schema",
            "encoding",
            "first_revision_id",
            "firstRevisionId",
            "last_revision_id",
            "lastRevisionId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Schema,
            Encoding,
            FirstRevisionId,
            LastRevisionId,
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
                            "schema" => Ok(GeneratedField::Schema),
                            "encoding" => Ok(GeneratedField::Encoding),
                            "firstRevisionId" | "first_revision_id" => Ok(GeneratedField::FirstRevisionId),
                            "lastRevisionId" | "last_revision_id" => Ok(GeneratedField::LastRevisionId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SchemaSettings;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.SchemaSettings")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<SchemaSettings, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut schema__ = None;
                let mut encoding__ = None;
                let mut first_revision_id__ = None;
                let mut last_revision_id__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Schema => {
                            if schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schema"));
                            }
                            schema__ = Some(map.next_value()?);
                        }
                        GeneratedField::Encoding => {
                            if encoding__.is_some() {
                                return Err(serde::de::Error::duplicate_field("encoding"));
                            }
                            encoding__ = Some(map.next_value::<Encoding>()? as i32);
                        }
                        GeneratedField::FirstRevisionId => {
                            if first_revision_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("firstRevisionId"));
                            }
                            first_revision_id__ = Some(map.next_value()?);
                        }
                        GeneratedField::LastRevisionId => {
                            if last_revision_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lastRevisionId"));
                            }
                            last_revision_id__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(SchemaSettings {
                    schema: schema__.unwrap_or_default(),
                    encoding: encoding__.unwrap_or_default(),
                    first_revision_id: first_revision_id__.unwrap_or_default(),
                    last_revision_id: last_revision_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.SchemaSettings", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SchemaView {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "SCHEMA_VIEW_UNSPECIFIED",
            Self::Basic => "BASIC",
            Self::Full => "FULL",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for SchemaView {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "SCHEMA_VIEW_UNSPECIFIED",
            "BASIC",
            "FULL",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SchemaView;

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
                    .and_then(SchemaView::from_i32)
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
                    .and_then(SchemaView::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "SCHEMA_VIEW_UNSPECIFIED" => Ok(SchemaView::Unspecified),
                    "BASIC" => Ok(SchemaView::Basic),
                    "FULL" => Ok(SchemaView::Full),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for SeekRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        if self.target.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.SeekRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        if let Some(v) = self.target.as_ref() {
            match v {
                seek_request::Target::Time(v) => {
                    struct_ser.serialize_field("time", v)?;
                }
                seek_request::Target::Snapshot(v) => {
                    struct_ser.serialize_field("snapshot", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SeekRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
            "time",
            "snapshot",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
            Time,
            Snapshot,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            "time" => Ok(GeneratedField::Time),
                            "snapshot" => Ok(GeneratedField::Snapshot),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SeekRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.SeekRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<SeekRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                let mut target__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                        GeneratedField::Time => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("time"));
                            }
                            target__ = map.next_value::<::std::option::Option<_>>()?.map(seek_request::Target::Time)
;
                        }
                        GeneratedField::Snapshot => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("snapshot"));
                            }
                            target__ = map.next_value::<::std::option::Option<_>>()?.map(seek_request::Target::Snapshot);
                        }
                    }
                }
                Ok(SeekRequest {
                    subscription: subscription__.unwrap_or_default(),
                    target: target__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.SeekRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SeekResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.pubsub.v1.SeekResponse", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SeekResponse {
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
            type Value = SeekResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.SeekResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<SeekResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(SeekResponse {
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.SeekResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Snapshot {
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
        if !self.topic.is_empty() {
            len += 1;
        }
        if self.expire_time.is_some() {
            len += 1;
        }
        if !self.labels.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.Snapshot", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.topic.is_empty() {
            struct_ser.serialize_field("topic", &self.topic)?;
        }
        if let Some(v) = self.expire_time.as_ref() {
            struct_ser.serialize_field("expireTime", v)?;
        }
        if !self.labels.is_empty() {
            struct_ser.serialize_field("labels", &self.labels)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Snapshot {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "topic",
            "expire_time",
            "expireTime",
            "labels",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Topic,
            ExpireTime,
            Labels,
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
                            "topic" => Ok(GeneratedField::Topic),
                            "expireTime" | "expire_time" => Ok(GeneratedField::ExpireTime),
                            "labels" => Ok(GeneratedField::Labels),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Snapshot;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.Snapshot")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Snapshot, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut topic__ = None;
                let mut expire_time__ = None;
                let mut labels__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Topic => {
                            if topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topic"));
                            }
                            topic__ = Some(map.next_value()?);
                        }
                        GeneratedField::ExpireTime => {
                            if expire_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("expireTime"));
                            }
                            expire_time__ = map.next_value()?;
                        }
                        GeneratedField::Labels => {
                            if labels__.is_some() {
                                return Err(serde::de::Error::duplicate_field("labels"));
                            }
                            labels__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(Snapshot {
                    name: name__.unwrap_or_default(),
                    topic: topic__.unwrap_or_default(),
                    expire_time: expire_time__,
                    labels: labels__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.Snapshot", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamingPullRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.subscription.is_empty() {
            len += 1;
        }
        if !self.ack_ids.is_empty() {
            len += 1;
        }
        if !self.modify_deadline_seconds.is_empty() {
            len += 1;
        }
        if !self.modify_deadline_ack_ids.is_empty() {
            len += 1;
        }
        if self.stream_ack_deadline_seconds != 0 {
            len += 1;
        }
        if !self.client_id.is_empty() {
            len += 1;
        }
        if self.max_outstanding_messages != 0 {
            len += 1;
        }
        if self.max_outstanding_bytes != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.StreamingPullRequest", len)?;
        if !self.subscription.is_empty() {
            struct_ser.serialize_field("subscription", &self.subscription)?;
        }
        if !self.ack_ids.is_empty() {
            struct_ser.serialize_field("ackIds", &self.ack_ids)?;
        }
        if !self.modify_deadline_seconds.is_empty() {
            struct_ser.serialize_field("modifyDeadlineSeconds", &self.modify_deadline_seconds)?;
        }
        if !self.modify_deadline_ack_ids.is_empty() {
            struct_ser.serialize_field("modifyDeadlineAckIds", &self.modify_deadline_ack_ids)?;
        }
        if self.stream_ack_deadline_seconds != 0 {
            struct_ser.serialize_field("streamAckDeadlineSeconds", &self.stream_ack_deadline_seconds)?;
        }
        if !self.client_id.is_empty() {
            struct_ser.serialize_field("clientId", &self.client_id)?;
        }
        if self.max_outstanding_messages != 0 {
            struct_ser.serialize_field("maxOutstandingMessages", ToString::to_string(&self.max_outstanding_messages).as_str())?;
        }
        if self.max_outstanding_bytes != 0 {
            struct_ser.serialize_field("maxOutstandingBytes", ToString::to_string(&self.max_outstanding_bytes).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamingPullRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
            "ack_ids",
            "ackIds",
            "modify_deadline_seconds",
            "modifyDeadlineSeconds",
            "modify_deadline_ack_ids",
            "modifyDeadlineAckIds",
            "stream_ack_deadline_seconds",
            "streamAckDeadlineSeconds",
            "client_id",
            "clientId",
            "max_outstanding_messages",
            "maxOutstandingMessages",
            "max_outstanding_bytes",
            "maxOutstandingBytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
            AckIds,
            ModifyDeadlineSeconds,
            ModifyDeadlineAckIds,
            StreamAckDeadlineSeconds,
            ClientId,
            MaxOutstandingMessages,
            MaxOutstandingBytes,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            "ackIds" | "ack_ids" => Ok(GeneratedField::AckIds),
                            "modifyDeadlineSeconds" | "modify_deadline_seconds" => Ok(GeneratedField::ModifyDeadlineSeconds),
                            "modifyDeadlineAckIds" | "modify_deadline_ack_ids" => Ok(GeneratedField::ModifyDeadlineAckIds),
                            "streamAckDeadlineSeconds" | "stream_ack_deadline_seconds" => Ok(GeneratedField::StreamAckDeadlineSeconds),
                            "clientId" | "client_id" => Ok(GeneratedField::ClientId),
                            "maxOutstandingMessages" | "max_outstanding_messages" => Ok(GeneratedField::MaxOutstandingMessages),
                            "maxOutstandingBytes" | "max_outstanding_bytes" => Ok(GeneratedField::MaxOutstandingBytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StreamingPullRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.StreamingPullRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<StreamingPullRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                let mut ack_ids__ = None;
                let mut modify_deadline_seconds__ = None;
                let mut modify_deadline_ack_ids__ = None;
                let mut stream_ack_deadline_seconds__ = None;
                let mut client_id__ = None;
                let mut max_outstanding_messages__ = None;
                let mut max_outstanding_bytes__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = Some(map.next_value()?);
                        }
                        GeneratedField::AckIds => {
                            if ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ackIds"));
                            }
                            ack_ids__ = Some(map.next_value()?);
                        }
                        GeneratedField::ModifyDeadlineSeconds => {
                            if modify_deadline_seconds__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modifyDeadlineSeconds"));
                            }
                            modify_deadline_seconds__ =
                                Some(map.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::ModifyDeadlineAckIds => {
                            if modify_deadline_ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modifyDeadlineAckIds"));
                            }
                            modify_deadline_ack_ids__ = Some(map.next_value()?);
                        }
                        GeneratedField::StreamAckDeadlineSeconds => {
                            if stream_ack_deadline_seconds__.is_some() {
                                return Err(serde::de::Error::duplicate_field("streamAckDeadlineSeconds"));
                            }
                            stream_ack_deadline_seconds__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ClientId => {
                            if client_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clientId"));
                            }
                            client_id__ = Some(map.next_value()?);
                        }
                        GeneratedField::MaxOutstandingMessages => {
                            if max_outstanding_messages__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxOutstandingMessages"));
                            }
                            max_outstanding_messages__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MaxOutstandingBytes => {
                            if max_outstanding_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxOutstandingBytes"));
                            }
                            max_outstanding_bytes__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(StreamingPullRequest {
                    subscription: subscription__.unwrap_or_default(),
                    ack_ids: ack_ids__.unwrap_or_default(),
                    modify_deadline_seconds: modify_deadline_seconds__.unwrap_or_default(),
                    modify_deadline_ack_ids: modify_deadline_ack_ids__.unwrap_or_default(),
                    stream_ack_deadline_seconds: stream_ack_deadline_seconds__.unwrap_or_default(),
                    client_id: client_id__.unwrap_or_default(),
                    max_outstanding_messages: max_outstanding_messages__.unwrap_or_default(),
                    max_outstanding_bytes: max_outstanding_bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.StreamingPullRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamingPullResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.received_messages.is_empty() {
            len += 1;
        }
        if self.acknowledge_confirmation.is_some() {
            len += 1;
        }
        if self.modify_ack_deadline_confirmation.is_some() {
            len += 1;
        }
        if self.subscription_properties.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.StreamingPullResponse", len)?;
        if !self.received_messages.is_empty() {
            struct_ser.serialize_field("receivedMessages", &self.received_messages)?;
        }
        if let Some(v) = self.acknowledge_confirmation.as_ref() {
            struct_ser.serialize_field("acknowledgeConfirmation", v)?;
        }
        if let Some(v) = self.modify_ack_deadline_confirmation.as_ref() {
            struct_ser.serialize_field("modifyAckDeadlineConfirmation", v)?;
        }
        if let Some(v) = self.subscription_properties.as_ref() {
            struct_ser.serialize_field("subscriptionProperties", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamingPullResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "received_messages",
            "receivedMessages",
            "acknowledge_confirmation",
            "acknowledgeConfirmation",
            "modify_ack_deadline_confirmation",
            "modifyAckDeadlineConfirmation",
            "subscription_properties",
            "subscriptionProperties",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ReceivedMessages,
            AcknowledgeConfirmation,
            ModifyAckDeadlineConfirmation,
            SubscriptionProperties,
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
                            "receivedMessages" | "received_messages" => Ok(GeneratedField::ReceivedMessages),
                            "acknowledgeConfirmation" | "acknowledge_confirmation" => Ok(GeneratedField::AcknowledgeConfirmation),
                            "modifyAckDeadlineConfirmation" | "modify_ack_deadline_confirmation" => Ok(GeneratedField::ModifyAckDeadlineConfirmation),
                            "subscriptionProperties" | "subscription_properties" => Ok(GeneratedField::SubscriptionProperties),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StreamingPullResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.StreamingPullResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<StreamingPullResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut received_messages__ = None;
                let mut acknowledge_confirmation__ = None;
                let mut modify_ack_deadline_confirmation__ = None;
                let mut subscription_properties__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ReceivedMessages => {
                            if received_messages__.is_some() {
                                return Err(serde::de::Error::duplicate_field("receivedMessages"));
                            }
                            received_messages__ = Some(map.next_value()?);
                        }
                        GeneratedField::AcknowledgeConfirmation => {
                            if acknowledge_confirmation__.is_some() {
                                return Err(serde::de::Error::duplicate_field("acknowledgeConfirmation"));
                            }
                            acknowledge_confirmation__ = map.next_value()?;
                        }
                        GeneratedField::ModifyAckDeadlineConfirmation => {
                            if modify_ack_deadline_confirmation__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modifyAckDeadlineConfirmation"));
                            }
                            modify_ack_deadline_confirmation__ = map.next_value()?;
                        }
                        GeneratedField::SubscriptionProperties => {
                            if subscription_properties__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscriptionProperties"));
                            }
                            subscription_properties__ = map.next_value()?;
                        }
                    }
                }
                Ok(StreamingPullResponse {
                    received_messages: received_messages__.unwrap_or_default(),
                    acknowledge_confirmation: acknowledge_confirmation__,
                    modify_ack_deadline_confirmation: modify_ack_deadline_confirmation__,
                    subscription_properties: subscription_properties__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.StreamingPullResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for streaming_pull_response::AcknowledgeConfirmation {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.ack_ids.is_empty() {
            len += 1;
        }
        if !self.invalid_ack_ids.is_empty() {
            len += 1;
        }
        if !self.unordered_ack_ids.is_empty() {
            len += 1;
        }
        if !self.temporary_failed_ack_ids.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.StreamingPullResponse.AcknowledgeConfirmation", len)?;
        if !self.ack_ids.is_empty() {
            struct_ser.serialize_field("ackIds", &self.ack_ids)?;
        }
        if !self.invalid_ack_ids.is_empty() {
            struct_ser.serialize_field("invalidAckIds", &self.invalid_ack_ids)?;
        }
        if !self.unordered_ack_ids.is_empty() {
            struct_ser.serialize_field("unorderedAckIds", &self.unordered_ack_ids)?;
        }
        if !self.temporary_failed_ack_ids.is_empty() {
            struct_ser.serialize_field("temporaryFailedAckIds", &self.temporary_failed_ack_ids)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for streaming_pull_response::AcknowledgeConfirmation {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ack_ids",
            "ackIds",
            "invalid_ack_ids",
            "invalidAckIds",
            "unordered_ack_ids",
            "unorderedAckIds",
            "temporary_failed_ack_ids",
            "temporaryFailedAckIds",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AckIds,
            InvalidAckIds,
            UnorderedAckIds,
            TemporaryFailedAckIds,
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
                            "ackIds" | "ack_ids" => Ok(GeneratedField::AckIds),
                            "invalidAckIds" | "invalid_ack_ids" => Ok(GeneratedField::InvalidAckIds),
                            "unorderedAckIds" | "unordered_ack_ids" => Ok(GeneratedField::UnorderedAckIds),
                            "temporaryFailedAckIds" | "temporary_failed_ack_ids" => Ok(GeneratedField::TemporaryFailedAckIds),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = streaming_pull_response::AcknowledgeConfirmation;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.StreamingPullResponse.AcknowledgeConfirmation")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<streaming_pull_response::AcknowledgeConfirmation, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ack_ids__ = None;
                let mut invalid_ack_ids__ = None;
                let mut unordered_ack_ids__ = None;
                let mut temporary_failed_ack_ids__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::AckIds => {
                            if ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ackIds"));
                            }
                            ack_ids__ = Some(map.next_value()?);
                        }
                        GeneratedField::InvalidAckIds => {
                            if invalid_ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("invalidAckIds"));
                            }
                            invalid_ack_ids__ = Some(map.next_value()?);
                        }
                        GeneratedField::UnorderedAckIds => {
                            if unordered_ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("unorderedAckIds"));
                            }
                            unordered_ack_ids__ = Some(map.next_value()?);
                        }
                        GeneratedField::TemporaryFailedAckIds => {
                            if temporary_failed_ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("temporaryFailedAckIds"));
                            }
                            temporary_failed_ack_ids__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(streaming_pull_response::AcknowledgeConfirmation {
                    ack_ids: ack_ids__.unwrap_or_default(),
                    invalid_ack_ids: invalid_ack_ids__.unwrap_or_default(),
                    unordered_ack_ids: unordered_ack_ids__.unwrap_or_default(),
                    temporary_failed_ack_ids: temporary_failed_ack_ids__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.StreamingPullResponse.AcknowledgeConfirmation", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for streaming_pull_response::ModifyAckDeadlineConfirmation {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.ack_ids.is_empty() {
            len += 1;
        }
        if !self.invalid_ack_ids.is_empty() {
            len += 1;
        }
        if !self.temporary_failed_ack_ids.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.StreamingPullResponse.ModifyAckDeadlineConfirmation", len)?;
        if !self.ack_ids.is_empty() {
            struct_ser.serialize_field("ackIds", &self.ack_ids)?;
        }
        if !self.invalid_ack_ids.is_empty() {
            struct_ser.serialize_field("invalidAckIds", &self.invalid_ack_ids)?;
        }
        if !self.temporary_failed_ack_ids.is_empty() {
            struct_ser.serialize_field("temporaryFailedAckIds", &self.temporary_failed_ack_ids)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for streaming_pull_response::ModifyAckDeadlineConfirmation {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ack_ids",
            "ackIds",
            "invalid_ack_ids",
            "invalidAckIds",
            "temporary_failed_ack_ids",
            "temporaryFailedAckIds",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AckIds,
            InvalidAckIds,
            TemporaryFailedAckIds,
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
                            "ackIds" | "ack_ids" => Ok(GeneratedField::AckIds),
                            "invalidAckIds" | "invalid_ack_ids" => Ok(GeneratedField::InvalidAckIds),
                            "temporaryFailedAckIds" | "temporary_failed_ack_ids" => Ok(GeneratedField::TemporaryFailedAckIds),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = streaming_pull_response::ModifyAckDeadlineConfirmation;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.StreamingPullResponse.ModifyAckDeadlineConfirmation")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<streaming_pull_response::ModifyAckDeadlineConfirmation, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ack_ids__ = None;
                let mut invalid_ack_ids__ = None;
                let mut temporary_failed_ack_ids__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::AckIds => {
                            if ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ackIds"));
                            }
                            ack_ids__ = Some(map.next_value()?);
                        }
                        GeneratedField::InvalidAckIds => {
                            if invalid_ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("invalidAckIds"));
                            }
                            invalid_ack_ids__ = Some(map.next_value()?);
                        }
                        GeneratedField::TemporaryFailedAckIds => {
                            if temporary_failed_ack_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("temporaryFailedAckIds"));
                            }
                            temporary_failed_ack_ids__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(streaming_pull_response::ModifyAckDeadlineConfirmation {
                    ack_ids: ack_ids__.unwrap_or_default(),
                    invalid_ack_ids: invalid_ack_ids__.unwrap_or_default(),
                    temporary_failed_ack_ids: temporary_failed_ack_ids__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.StreamingPullResponse.ModifyAckDeadlineConfirmation", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for streaming_pull_response::SubscriptionProperties {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.exactly_once_delivery_enabled {
            len += 1;
        }
        if self.message_ordering_enabled {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.StreamingPullResponse.SubscriptionProperties", len)?;
        if self.exactly_once_delivery_enabled {
            struct_ser.serialize_field("exactlyOnceDeliveryEnabled", &self.exactly_once_delivery_enabled)?;
        }
        if self.message_ordering_enabled {
            struct_ser.serialize_field("messageOrderingEnabled", &self.message_ordering_enabled)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for streaming_pull_response::SubscriptionProperties {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "exactly_once_delivery_enabled",
            "exactlyOnceDeliveryEnabled",
            "message_ordering_enabled",
            "messageOrderingEnabled",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ExactlyOnceDeliveryEnabled,
            MessageOrderingEnabled,
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
                            "exactlyOnceDeliveryEnabled" | "exactly_once_delivery_enabled" => Ok(GeneratedField::ExactlyOnceDeliveryEnabled),
                            "messageOrderingEnabled" | "message_ordering_enabled" => Ok(GeneratedField::MessageOrderingEnabled),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = streaming_pull_response::SubscriptionProperties;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.StreamingPullResponse.SubscriptionProperties")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<streaming_pull_response::SubscriptionProperties, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut exactly_once_delivery_enabled__ = None;
                let mut message_ordering_enabled__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ExactlyOnceDeliveryEnabled => {
                            if exactly_once_delivery_enabled__.is_some() {
                                return Err(serde::de::Error::duplicate_field("exactlyOnceDeliveryEnabled"));
                            }
                            exactly_once_delivery_enabled__ = Some(map.next_value()?);
                        }
                        GeneratedField::MessageOrderingEnabled => {
                            if message_ordering_enabled__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messageOrderingEnabled"));
                            }
                            message_ordering_enabled__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(streaming_pull_response::SubscriptionProperties {
                    exactly_once_delivery_enabled: exactly_once_delivery_enabled__.unwrap_or_default(),
                    message_ordering_enabled: message_ordering_enabled__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.StreamingPullResponse.SubscriptionProperties", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Subscription {
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
        if !self.topic.is_empty() {
            len += 1;
        }
        if self.push_config.is_some() {
            len += 1;
        }
        if self.bigquery_config.is_some() {
            len += 1;
        }
        if self.cloud_storage_config.is_some() {
            len += 1;
        }
        if self.ack_deadline_seconds != 0 {
            len += 1;
        }
        if self.retain_acked_messages {
            len += 1;
        }
        if self.message_retention_duration.is_some() {
            len += 1;
        }
        if !self.labels.is_empty() {
            len += 1;
        }
        if self.enable_message_ordering {
            len += 1;
        }
        if self.expiration_policy.is_some() {
            len += 1;
        }
        if !self.filter.is_empty() {
            len += 1;
        }
        if self.dead_letter_policy.is_some() {
            len += 1;
        }
        if self.retry_policy.is_some() {
            len += 1;
        }
        if self.detached {
            len += 1;
        }
        if self.enable_exactly_once_delivery {
            len += 1;
        }
        if self.topic_message_retention_duration.is_some() {
            len += 1;
        }
        if self.state != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.Subscription", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.topic.is_empty() {
            struct_ser.serialize_field("topic", &self.topic)?;
        }
        if let Some(v) = self.push_config.as_ref() {
            struct_ser.serialize_field("pushConfig", v)?;
        }
        if let Some(v) = self.bigquery_config.as_ref() {
            struct_ser.serialize_field("bigqueryConfig", v)?;
        }
        if let Some(v) = self.cloud_storage_config.as_ref() {
            struct_ser.serialize_field("cloudStorageConfig", v)?;
        }
        if self.ack_deadline_seconds != 0 {
            struct_ser.serialize_field("ackDeadlineSeconds", &self.ack_deadline_seconds)?;
        }
        if self.retain_acked_messages {
            struct_ser.serialize_field("retainAckedMessages", &self.retain_acked_messages)?;
        }
        if let Some(v) = self.message_retention_duration.as_ref() {
            struct_ser.serialize_field("messageRetentionDuration", v)?;
        }
        if !self.labels.is_empty() {
            struct_ser.serialize_field("labels", &self.labels)?;
        }
        if self.enable_message_ordering {
            struct_ser.serialize_field("enableMessageOrdering", &self.enable_message_ordering)?;
        }
        if let Some(v) = self.expiration_policy.as_ref() {
            struct_ser.serialize_field("expirationPolicy", v)?;
        }
        if !self.filter.is_empty() {
            struct_ser.serialize_field("filter", &self.filter)?;
        }
        if let Some(v) = self.dead_letter_policy.as_ref() {
            struct_ser.serialize_field("deadLetterPolicy", v)?;
        }
        if let Some(v) = self.retry_policy.as_ref() {
            struct_ser.serialize_field("retryPolicy", v)?;
        }
        if self.detached {
            struct_ser.serialize_field("detached", &self.detached)?;
        }
        if self.enable_exactly_once_delivery {
            struct_ser.serialize_field("enableExactlyOnceDelivery", &self.enable_exactly_once_delivery)?;
        }
        if let Some(v) = self.topic_message_retention_duration.as_ref() {
            struct_ser.serialize_field("topicMessageRetentionDuration", v)?;
        }
        if self.state != 0 {
            let v = subscription::State::from_i32(self.state)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.state)))?;
            struct_ser.serialize_field("state", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Subscription {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "topic",
            "push_config",
            "pushConfig",
            "bigquery_config",
            "bigqueryConfig",
            "cloud_storage_config",
            "cloudStorageConfig",
            "ack_deadline_seconds",
            "ackDeadlineSeconds",
            "retain_acked_messages",
            "retainAckedMessages",
            "message_retention_duration",
            "messageRetentionDuration",
            "labels",
            "enable_message_ordering",
            "enableMessageOrdering",
            "expiration_policy",
            "expirationPolicy",
            "filter",
            "dead_letter_policy",
            "deadLetterPolicy",
            "retry_policy",
            "retryPolicy",
            "detached",
            "enable_exactly_once_delivery",
            "enableExactlyOnceDelivery",
            "topic_message_retention_duration",
            "topicMessageRetentionDuration",
            "state",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Topic,
            PushConfig,
            BigqueryConfig,
            CloudStorageConfig,
            AckDeadlineSeconds,
            RetainAckedMessages,
            MessageRetentionDuration,
            Labels,
            EnableMessageOrdering,
            ExpirationPolicy,
            Filter,
            DeadLetterPolicy,
            RetryPolicy,
            Detached,
            EnableExactlyOnceDelivery,
            TopicMessageRetentionDuration,
            State,
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
                            "topic" => Ok(GeneratedField::Topic),
                            "pushConfig" | "push_config" => Ok(GeneratedField::PushConfig),
                            "bigqueryConfig" | "bigquery_config" => Ok(GeneratedField::BigqueryConfig),
                            "cloudStorageConfig" | "cloud_storage_config" => Ok(GeneratedField::CloudStorageConfig),
                            "ackDeadlineSeconds" | "ack_deadline_seconds" => Ok(GeneratedField::AckDeadlineSeconds),
                            "retainAckedMessages" | "retain_acked_messages" => Ok(GeneratedField::RetainAckedMessages),
                            "messageRetentionDuration" | "message_retention_duration" => Ok(GeneratedField::MessageRetentionDuration),
                            "labels" => Ok(GeneratedField::Labels),
                            "enableMessageOrdering" | "enable_message_ordering" => Ok(GeneratedField::EnableMessageOrdering),
                            "expirationPolicy" | "expiration_policy" => Ok(GeneratedField::ExpirationPolicy),
                            "filter" => Ok(GeneratedField::Filter),
                            "deadLetterPolicy" | "dead_letter_policy" => Ok(GeneratedField::DeadLetterPolicy),
                            "retryPolicy" | "retry_policy" => Ok(GeneratedField::RetryPolicy),
                            "detached" => Ok(GeneratedField::Detached),
                            "enableExactlyOnceDelivery" | "enable_exactly_once_delivery" => Ok(GeneratedField::EnableExactlyOnceDelivery),
                            "topicMessageRetentionDuration" | "topic_message_retention_duration" => Ok(GeneratedField::TopicMessageRetentionDuration),
                            "state" => Ok(GeneratedField::State),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Subscription;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.Subscription")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Subscription, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut topic__ = None;
                let mut push_config__ = None;
                let mut bigquery_config__ = None;
                let mut cloud_storage_config__ = None;
                let mut ack_deadline_seconds__ = None;
                let mut retain_acked_messages__ = None;
                let mut message_retention_duration__ = None;
                let mut labels__ = None;
                let mut enable_message_ordering__ = None;
                let mut expiration_policy__ = None;
                let mut filter__ = None;
                let mut dead_letter_policy__ = None;
                let mut retry_policy__ = None;
                let mut detached__ = None;
                let mut enable_exactly_once_delivery__ = None;
                let mut topic_message_retention_duration__ = None;
                let mut state__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Topic => {
                            if topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topic"));
                            }
                            topic__ = Some(map.next_value()?);
                        }
                        GeneratedField::PushConfig => {
                            if push_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pushConfig"));
                            }
                            push_config__ = map.next_value()?;
                        }
                        GeneratedField::BigqueryConfig => {
                            if bigquery_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bigqueryConfig"));
                            }
                            bigquery_config__ = map.next_value()?;
                        }
                        GeneratedField::CloudStorageConfig => {
                            if cloud_storage_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cloudStorageConfig"));
                            }
                            cloud_storage_config__ = map.next_value()?;
                        }
                        GeneratedField::AckDeadlineSeconds => {
                            if ack_deadline_seconds__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ackDeadlineSeconds"));
                            }
                            ack_deadline_seconds__ =
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RetainAckedMessages => {
                            if retain_acked_messages__.is_some() {
                                return Err(serde::de::Error::duplicate_field("retainAckedMessages"));
                            }
                            retain_acked_messages__ = Some(map.next_value()?);
                        }
                        GeneratedField::MessageRetentionDuration => {
                            if message_retention_duration__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messageRetentionDuration"));
                            }
                            message_retention_duration__ = map.next_value()?;
                        }
                        GeneratedField::Labels => {
                            if labels__.is_some() {
                                return Err(serde::de::Error::duplicate_field("labels"));
                            }
                            labels__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::EnableMessageOrdering => {
                            if enable_message_ordering__.is_some() {
                                return Err(serde::de::Error::duplicate_field("enableMessageOrdering"));
                            }
                            enable_message_ordering__ = Some(map.next_value()?);
                        }
                        GeneratedField::ExpirationPolicy => {
                            if expiration_policy__.is_some() {
                                return Err(serde::de::Error::duplicate_field("expirationPolicy"));
                            }
                            expiration_policy__ = map.next_value()?;
                        }
                        GeneratedField::Filter => {
                            if filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("filter"));
                            }
                            filter__ = Some(map.next_value()?);
                        }
                        GeneratedField::DeadLetterPolicy => {
                            if dead_letter_policy__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deadLetterPolicy"));
                            }
                            dead_letter_policy__ = map.next_value()?;
                        }
                        GeneratedField::RetryPolicy => {
                            if retry_policy__.is_some() {
                                return Err(serde::de::Error::duplicate_field("retryPolicy"));
                            }
                            retry_policy__ = map.next_value()?;
                        }
                        GeneratedField::Detached => {
                            if detached__.is_some() {
                                return Err(serde::de::Error::duplicate_field("detached"));
                            }
                            detached__ = Some(map.next_value()?);
                        }
                        GeneratedField::EnableExactlyOnceDelivery => {
                            if enable_exactly_once_delivery__.is_some() {
                                return Err(serde::de::Error::duplicate_field("enableExactlyOnceDelivery"));
                            }
                            enable_exactly_once_delivery__ = Some(map.next_value()?);
                        }
                        GeneratedField::TopicMessageRetentionDuration => {
                            if topic_message_retention_duration__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topicMessageRetentionDuration"));
                            }
                            topic_message_retention_duration__ = map.next_value()?;
                        }
                        GeneratedField::State => {
                            if state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state__ = Some(map.next_value::<subscription::State>()? as i32);
                        }
                    }
                }
                Ok(Subscription {
                    name: name__.unwrap_or_default(),
                    topic: topic__.unwrap_or_default(),
                    push_config: push_config__,
                    bigquery_config: bigquery_config__,
                    cloud_storage_config: cloud_storage_config__,
                    ack_deadline_seconds: ack_deadline_seconds__.unwrap_or_default(),
                    retain_acked_messages: retain_acked_messages__.unwrap_or_default(),
                    message_retention_duration: message_retention_duration__,
                    labels: labels__.unwrap_or_default(),
                    enable_message_ordering: enable_message_ordering__.unwrap_or_default(),
                    expiration_policy: expiration_policy__,
                    filter: filter__.unwrap_or_default(),
                    dead_letter_policy: dead_letter_policy__,
                    retry_policy: retry_policy__,
                    detached: detached__.unwrap_or_default(),
                    enable_exactly_once_delivery: enable_exactly_once_delivery__.unwrap_or_default(),
                    topic_message_retention_duration: topic_message_retention_duration__,
                    state: state__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.Subscription", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for subscription::State {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "STATE_UNSPECIFIED",
            Self::Active => "ACTIVE",
            Self::ResourceError => "RESOURCE_ERROR",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for subscription::State {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "STATE_UNSPECIFIED",
            "ACTIVE",
            "RESOURCE_ERROR",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = subscription::State;

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
                    .and_then(subscription::State::from_i32)
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
                    .and_then(subscription::State::from_i32)
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "STATE_UNSPECIFIED" => Ok(subscription::State::Unspecified),
                    "ACTIVE" => Ok(subscription::State::Active),
                    "RESOURCE_ERROR" => Ok(subscription::State::ResourceError),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Topic {
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
        if !self.labels.is_empty() {
            len += 1;
        }
        if self.message_storage_policy.is_some() {
            len += 1;
        }
        if !self.kms_key_name.is_empty() {
            len += 1;
        }
        if self.schema_settings.is_some() {
            len += 1;
        }
        if self.satisfies_pzs {
            len += 1;
        }
        if self.message_retention_duration.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.Topic", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.labels.is_empty() {
            struct_ser.serialize_field("labels", &self.labels)?;
        }
        if let Some(v) = self.message_storage_policy.as_ref() {
            struct_ser.serialize_field("messageStoragePolicy", v)?;
        }
        if !self.kms_key_name.is_empty() {
            struct_ser.serialize_field("kmsKeyName", &self.kms_key_name)?;
        }
        if let Some(v) = self.schema_settings.as_ref() {
            struct_ser.serialize_field("schemaSettings", v)?;
        }
        if self.satisfies_pzs {
            struct_ser.serialize_field("satisfiesPzs", &self.satisfies_pzs)?;
        }
        if let Some(v) = self.message_retention_duration.as_ref() {
            struct_ser.serialize_field("messageRetentionDuration", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Topic {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "labels",
            "message_storage_policy",
            "messageStoragePolicy",
            "kms_key_name",
            "kmsKeyName",
            "schema_settings",
            "schemaSettings",
            "satisfies_pzs",
            "satisfiesPzs",
            "message_retention_duration",
            "messageRetentionDuration",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Labels,
            MessageStoragePolicy,
            KmsKeyName,
            SchemaSettings,
            SatisfiesPzs,
            MessageRetentionDuration,
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
                            "labels" => Ok(GeneratedField::Labels),
                            "messageStoragePolicy" | "message_storage_policy" => Ok(GeneratedField::MessageStoragePolicy),
                            "kmsKeyName" | "kms_key_name" => Ok(GeneratedField::KmsKeyName),
                            "schemaSettings" | "schema_settings" => Ok(GeneratedField::SchemaSettings),
                            "satisfiesPzs" | "satisfies_pzs" => Ok(GeneratedField::SatisfiesPzs),
                            "messageRetentionDuration" | "message_retention_duration" => Ok(GeneratedField::MessageRetentionDuration),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Topic;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.Topic")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Topic, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut labels__ = None;
                let mut message_storage_policy__ = None;
                let mut kms_key_name__ = None;
                let mut schema_settings__ = None;
                let mut satisfies_pzs__ = None;
                let mut message_retention_duration__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map.next_value()?);
                        }
                        GeneratedField::Labels => {
                            if labels__.is_some() {
                                return Err(serde::de::Error::duplicate_field("labels"));
                            }
                            labels__ = Some(
                                map.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::MessageStoragePolicy => {
                            if message_storage_policy__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messageStoragePolicy"));
                            }
                            message_storage_policy__ = map.next_value()?;
                        }
                        GeneratedField::KmsKeyName => {
                            if kms_key_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kmsKeyName"));
                            }
                            kms_key_name__ = Some(map.next_value()?);
                        }
                        GeneratedField::SchemaSettings => {
                            if schema_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schemaSettings"));
                            }
                            schema_settings__ = map.next_value()?;
                        }
                        GeneratedField::SatisfiesPzs => {
                            if satisfies_pzs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("satisfiesPzs"));
                            }
                            satisfies_pzs__ = Some(map.next_value()?);
                        }
                        GeneratedField::MessageRetentionDuration => {
                            if message_retention_duration__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messageRetentionDuration"));
                            }
                            message_retention_duration__ = map.next_value()?;
                        }
                    }
                }
                Ok(Topic {
                    name: name__.unwrap_or_default(),
                    labels: labels__.unwrap_or_default(),
                    message_storage_policy: message_storage_policy__,
                    kms_key_name: kms_key_name__.unwrap_or_default(),
                    schema_settings: schema_settings__,
                    satisfies_pzs: satisfies_pzs__.unwrap_or_default(),
                    message_retention_duration: message_retention_duration__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.Topic", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UpdateSnapshotRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.snapshot.is_some() {
            len += 1;
        }
        if self.update_mask.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.UpdateSnapshotRequest", len)?;
        if let Some(v) = self.snapshot.as_ref() {
            struct_ser.serialize_field("snapshot", v)?;
        }
        if let Some(v) = self.update_mask.as_ref() {
            struct_ser.serialize_field("updateMask", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UpdateSnapshotRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "snapshot",
            "update_mask",
            "updateMask",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Snapshot,
            UpdateMask,
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
                            "snapshot" => Ok(GeneratedField::Snapshot),
                            "updateMask" | "update_mask" => Ok(GeneratedField::UpdateMask),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UpdateSnapshotRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.UpdateSnapshotRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<UpdateSnapshotRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut snapshot__ = None;
                let mut update_mask__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Snapshot => {
                            if snapshot__.is_some() {
                                return Err(serde::de::Error::duplicate_field("snapshot"));
                            }
                            snapshot__ = map.next_value()?;
                        }
                        GeneratedField::UpdateMask => {
                            if update_mask__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updateMask"));
                            }
                            update_mask__ = map.next_value()?;
                        }
                    }
                }
                Ok(UpdateSnapshotRequest {
                    snapshot: snapshot__,
                    update_mask: update_mask__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.UpdateSnapshotRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UpdateSubscriptionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.subscription.is_some() {
            len += 1;
        }
        if self.update_mask.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.UpdateSubscriptionRequest", len)?;
        if let Some(v) = self.subscription.as_ref() {
            struct_ser.serialize_field("subscription", v)?;
        }
        if let Some(v) = self.update_mask.as_ref() {
            struct_ser.serialize_field("updateMask", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UpdateSubscriptionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "subscription",
            "update_mask",
            "updateMask",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Subscription,
            UpdateMask,
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
                            "subscription" => Ok(GeneratedField::Subscription),
                            "updateMask" | "update_mask" => Ok(GeneratedField::UpdateMask),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UpdateSubscriptionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.UpdateSubscriptionRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<UpdateSubscriptionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut subscription__ = None;
                let mut update_mask__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Subscription => {
                            if subscription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("subscription"));
                            }
                            subscription__ = map.next_value()?;
                        }
                        GeneratedField::UpdateMask => {
                            if update_mask__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updateMask"));
                            }
                            update_mask__ = map.next_value()?;
                        }
                    }
                }
                Ok(UpdateSubscriptionRequest {
                    subscription: subscription__,
                    update_mask: update_mask__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.UpdateSubscriptionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UpdateTopicRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.topic.is_some() {
            len += 1;
        }
        if self.update_mask.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.UpdateTopicRequest", len)?;
        if let Some(v) = self.topic.as_ref() {
            struct_ser.serialize_field("topic", v)?;
        }
        if let Some(v) = self.update_mask.as_ref() {
            struct_ser.serialize_field("updateMask", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UpdateTopicRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "topic",
            "update_mask",
            "updateMask",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Topic,
            UpdateMask,
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
                            "topic" => Ok(GeneratedField::Topic),
                            "updateMask" | "update_mask" => Ok(GeneratedField::UpdateMask),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UpdateTopicRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.UpdateTopicRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<UpdateTopicRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut topic__ = None;
                let mut update_mask__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Topic => {
                            if topic__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topic"));
                            }
                            topic__ = map.next_value()?;
                        }
                        GeneratedField::UpdateMask => {
                            if update_mask__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updateMask"));
                            }
                            update_mask__ = map.next_value()?;
                        }
                    }
                }
                Ok(UpdateTopicRequest {
                    topic: topic__,
                    update_mask: update_mask__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.UpdateTopicRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ValidateMessageRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.parent.is_empty() {
            len += 1;
        }
        if !self.message.is_empty() {
            len += 1;
        }
        if self.encoding != 0 {
            len += 1;
        }
        if self.schema_spec.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ValidateMessageRequest", len)?;
        if !self.parent.is_empty() {
            struct_ser.serialize_field("parent", &self.parent)?;
        }
        if !self.message.is_empty() {
            struct_ser.serialize_field("message", pbjson::private::base64::encode(&self.message).as_str())?;
        }
        if self.encoding != 0 {
            let v = Encoding::from_i32(self.encoding)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", self.encoding)))?;
            struct_ser.serialize_field("encoding", &v)?;
        }
        if let Some(v) = self.schema_spec.as_ref() {
            match v {
                validate_message_request::SchemaSpec::Name(v) => {
                    struct_ser.serialize_field("name", v)?;
                }
                validate_message_request::SchemaSpec::Schema(v) => {
                    struct_ser.serialize_field("schema", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ValidateMessageRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "parent",
            "message",
            "encoding",
            "name",
            "schema",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Parent,
            Message,
            Encoding,
            Name,
            Schema,
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
                            "parent" => Ok(GeneratedField::Parent),
                            "message" => Ok(GeneratedField::Message),
                            "encoding" => Ok(GeneratedField::Encoding),
                            "name" => Ok(GeneratedField::Name),
                            "schema" => Ok(GeneratedField::Schema),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ValidateMessageRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ValidateMessageRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ValidateMessageRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut parent__ = None;
                let mut message__ = None;
                let mut encoding__ = None;
                let mut schema_spec__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Parent => {
                            if parent__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parent"));
                            }
                            parent__ = Some(map.next_value()?);
                        }
                        GeneratedField::Message => {
                            if message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message__ =
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Encoding => {
                            if encoding__.is_some() {
                                return Err(serde::de::Error::duplicate_field("encoding"));
                            }
                            encoding__ = Some(map.next_value::<Encoding>()? as i32);
                        }
                        GeneratedField::Name => {
                            if schema_spec__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            schema_spec__ = map.next_value::<::std::option::Option<_>>()?.map(validate_message_request::SchemaSpec::Name);
                        }
                        GeneratedField::Schema => {
                            if schema_spec__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schema"));
                            }
                            schema_spec__ = map.next_value::<::std::option::Option<_>>()?.map(validate_message_request::SchemaSpec::Schema)
;
                        }
                    }
                }
                Ok(ValidateMessageRequest {
                    parent: parent__.unwrap_or_default(),
                    message: message__.unwrap_or_default(),
                    encoding: encoding__.unwrap_or_default(),
                    schema_spec: schema_spec__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ValidateMessageRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ValidateMessageResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.pubsub.v1.ValidateMessageResponse", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ValidateMessageResponse {
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
            type Value = ValidateMessageResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ValidateMessageResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ValidateMessageResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(ValidateMessageResponse {
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ValidateMessageResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ValidateSchemaRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.parent.is_empty() {
            len += 1;
        }
        if self.schema.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.pubsub.v1.ValidateSchemaRequest", len)?;
        if !self.parent.is_empty() {
            struct_ser.serialize_field("parent", &self.parent)?;
        }
        if let Some(v) = self.schema.as_ref() {
            struct_ser.serialize_field("schema", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ValidateSchemaRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "parent",
            "schema",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Parent,
            Schema,
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
                            "parent" => Ok(GeneratedField::Parent),
                            "schema" => Ok(GeneratedField::Schema),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ValidateSchemaRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ValidateSchemaRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ValidateSchemaRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut parent__ = None;
                let mut schema__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Parent => {
                            if parent__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parent"));
                            }
                            parent__ = Some(map.next_value()?);
                        }
                        GeneratedField::Schema => {
                            if schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schema"));
                            }
                            schema__ = map.next_value()?;
                        }
                    }
                }
                Ok(ValidateSchemaRequest {
                    parent: parent__.unwrap_or_default(),
                    schema: schema__,
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ValidateSchemaRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ValidateSchemaResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.pubsub.v1.ValidateSchemaResponse", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ValidateSchemaResponse {
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
            type Value = ValidateSchemaResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.pubsub.v1.ValidateSchemaResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ValidateSchemaResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map.next_key::<GeneratedField>()?.is_some() {
                    let _ = map.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(ValidateSchemaResponse {
                })
            }
        }
        deserializer.deserialize_struct("google.pubsub.v1.ValidateSchemaResponse", FIELDS, GeneratedVisitor)
    }
}
