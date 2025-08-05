// @generated
impl serde::Serialize for ApiFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.filter.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.APIFilter", len)?;
        if let Some(v) = self.filter.as_ref() {
            match v {
                api_filter::Filter::TransactionRootFilter(v) => {
                    struct_ser.serialize_field("transactionRootFilter", v)?;
                }
                api_filter::Filter::UserTransactionFilter(v) => {
                    struct_ser.serialize_field("userTransactionFilter", v)?;
                }
                api_filter::Filter::EventFilter(v) => {
                    struct_ser.serialize_field("eventFilter", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ApiFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction_root_filter",
            "transactionRootFilter",
            "user_transaction_filter",
            "userTransactionFilter",
            "event_filter",
            "eventFilter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionRootFilter,
            UserTransactionFilter,
            EventFilter,
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
                            "transactionRootFilter" | "transaction_root_filter" => Ok(GeneratedField::TransactionRootFilter),
                            "userTransactionFilter" | "user_transaction_filter" => Ok(GeneratedField::UserTransactionFilter),
                            "eventFilter" | "event_filter" => Ok(GeneratedField::EventFilter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ApiFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.APIFilter")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ApiFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut filter__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::TransactionRootFilter => {
                            if filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionRootFilter"));
                            }
                            filter__ = map.next_value::<::std::option::Option<_>>()?.map(api_filter::Filter::TransactionRootFilter)
;
                        }
                        GeneratedField::UserTransactionFilter => {
                            if filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("userTransactionFilter"));
                            }
                            filter__ = map.next_value::<::std::option::Option<_>>()?.map(api_filter::Filter::UserTransactionFilter)
;
                        }
                        GeneratedField::EventFilter => {
                            if filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("eventFilter"));
                            }
                            filter__ = map.next_value::<::std::option::Option<_>>()?.map(api_filter::Filter::EventFilter)
;
                        }
                    }
                }
                Ok(ApiFilter {
                    filter: filter__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.APIFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ActiveStream {
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
        if self.start_time.is_some() {
            len += 1;
        }
        if self.start_version != 0 {
            len += 1;
        }
        if self.end_version.is_some() {
            len += 1;
        }
        if self.progress.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.ActiveStream", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.start_time.as_ref() {
            struct_ser.serialize_field("startTime", v)?;
        }
        if self.start_version != 0 {
            struct_ser.serialize_field("startVersion", ToString::to_string(&self.start_version).as_str())?;
        }
        if let Some(v) = self.end_version.as_ref() {
            struct_ser.serialize_field("endVersion", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.progress.as_ref() {
            struct_ser.serialize_field("progress", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ActiveStream {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "start_time",
            "startTime",
            "start_version",
            "startVersion",
            "end_version",
            "endVersion",
            "progress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            StartTime,
            StartVersion,
            EndVersion,
            Progress,
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
                            "startTime" | "start_time" => Ok(GeneratedField::StartTime),
                            "startVersion" | "start_version" => Ok(GeneratedField::StartVersion),
                            "endVersion" | "end_version" => Ok(GeneratedField::EndVersion),
                            "progress" => Ok(GeneratedField::Progress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ActiveStream;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.ActiveStream")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ActiveStream, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut start_time__ = None;
                let mut start_version__ = None;
                let mut end_version__ = None;
                let mut progress__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map.next_value()?);
                        }
                        GeneratedField::StartTime => {
                            if start_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTime"));
                            }
                            start_time__ = map.next_value()?;
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
                        GeneratedField::Progress => {
                            if progress__.is_some() {
                                return Err(serde::de::Error::duplicate_field("progress"));
                            }
                            progress__ = map.next_value()?;
                        }
                    }
                }
                Ok(ActiveStream {
                    id: id__.unwrap_or_default(),
                    start_time: start_time__,
                    start_version: start_version__.unwrap_or_default(),
                    end_version: end_version__,
                    progress: progress__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.ActiveStream", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BooleanTransactionFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.filter.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.BooleanTransactionFilter", len)?;
        if let Some(v) = self.filter.as_ref() {
            match v {
                boolean_transaction_filter::Filter::ApiFilter(v) => {
                    struct_ser.serialize_field("apiFilter", v)?;
                }
                boolean_transaction_filter::Filter::LogicalAnd(v) => {
                    struct_ser.serialize_field("logicalAnd", v)?;
                }
                boolean_transaction_filter::Filter::LogicalOr(v) => {
                    struct_ser.serialize_field("logicalOr", v)?;
                }
                boolean_transaction_filter::Filter::LogicalNot(v) => {
                    struct_ser.serialize_field("logicalNot", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BooleanTransactionFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "api_filter",
            "apiFilter",
            "logical_and",
            "logicalAnd",
            "logical_or",
            "logicalOr",
            "logical_not",
            "logicalNot",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ApiFilter,
            LogicalAnd,
            LogicalOr,
            LogicalNot,
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
                            "apiFilter" | "api_filter" => Ok(GeneratedField::ApiFilter),
                            "logicalAnd" | "logical_and" => Ok(GeneratedField::LogicalAnd),
                            "logicalOr" | "logical_or" => Ok(GeneratedField::LogicalOr),
                            "logicalNot" | "logical_not" => Ok(GeneratedField::LogicalNot),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BooleanTransactionFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.BooleanTransactionFilter")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<BooleanTransactionFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut filter__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ApiFilter => {
                            if filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("apiFilter"));
                            }
                            filter__ = map.next_value::<::std::option::Option<_>>()?.map(boolean_transaction_filter::Filter::ApiFilter)
;
                        }
                        GeneratedField::LogicalAnd => {
                            if filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("logicalAnd"));
                            }
                            filter__ = map.next_value::<::std::option::Option<_>>()?.map(boolean_transaction_filter::Filter::LogicalAnd)
;
                        }
                        GeneratedField::LogicalOr => {
                            if filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("logicalOr"));
                            }
                            filter__ = map.next_value::<::std::option::Option<_>>()?.map(boolean_transaction_filter::Filter::LogicalOr)
;
                        }
                        GeneratedField::LogicalNot => {
                            if filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("logicalNot"));
                            }
                            filter__ = map.next_value::<::std::option::Option<_>>()?.map(boolean_transaction_filter::Filter::LogicalNot)
;
                        }
                    }
                }
                Ok(BooleanTransactionFilter {
                    filter: filter__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.BooleanTransactionFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EntryFunctionFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.address.is_some() {
            len += 1;
        }
        if self.module_name.is_some() {
            len += 1;
        }
        if self.function.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.EntryFunctionFilter", len)?;
        if let Some(v) = self.address.as_ref() {
            struct_ser.serialize_field("address", v)?;
        }
        if let Some(v) = self.module_name.as_ref() {
            struct_ser.serialize_field("moduleName", v)?;
        }
        if let Some(v) = self.function.as_ref() {
            struct_ser.serialize_field("function", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EntryFunctionFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "module_name",
            "moduleName",
            "function",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            ModuleName,
            Function,
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
                            "moduleName" | "module_name" => Ok(GeneratedField::ModuleName),
                            "function" => Ok(GeneratedField::Function),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EntryFunctionFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.EntryFunctionFilter")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EntryFunctionFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut module_name__ = None;
                let mut function__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = map.next_value()?;
                        }
                        GeneratedField::ModuleName => {
                            if module_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("moduleName"));
                            }
                            module_name__ = map.next_value()?;
                        }
                        GeneratedField::Function => {
                            if function__.is_some() {
                                return Err(serde::de::Error::duplicate_field("function"));
                            }
                            function__ = map.next_value()?;
                        }
                    }
                }
                Ok(EntryFunctionFilter {
                    address: address__,
                    module_name: module_name__,
                    function: function__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.EntryFunctionFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EventFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.struct_type.is_some() {
            len += 1;
        }
        if self.data_substring_filter.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.EventFilter", len)?;
        if let Some(v) = self.struct_type.as_ref() {
            struct_ser.serialize_field("structType", v)?;
        }
        if let Some(v) = self.data_substring_filter.as_ref() {
            struct_ser.serialize_field("dataSubstringFilter", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EventFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "struct_type",
            "structType",
            "data_substring_filter",
            "dataSubstringFilter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StructType,
            DataSubstringFilter,
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
                            "structType" | "struct_type" => Ok(GeneratedField::StructType),
                            "dataSubstringFilter" | "data_substring_filter" => Ok(GeneratedField::DataSubstringFilter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EventFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.EventFilter")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut struct_type__ = None;
                let mut data_substring_filter__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::StructType => {
                            if struct_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("structType"));
                            }
                            struct_type__ = map.next_value()?;
                        }
                        GeneratedField::DataSubstringFilter => {
                            if data_substring_filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dataSubstringFilter"));
                            }
                            data_substring_filter__ = map.next_value()?;
                        }
                    }
                }
                Ok(EventFilter {
                    struct_type: struct_type__,
                    data_substring_filter: data_substring_filter__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.EventFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EventWithMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.event.is_some() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.version != 0 {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        if self.success {
            len += 1;
        }
        if !self.vm_status.is_empty() {
            len += 1;
        }
        if self.block_height != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.EventWithMetadata", len)?;
        if let Some(v) = self.event.as_ref() {
            struct_ser.serialize_field("event", v)?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", pbjson::private::base64::encode(&self.hash).as_str())?;
        }
        if self.success {
            struct_ser.serialize_field("success", &self.success)?;
        }
        if !self.vm_status.is_empty() {
            struct_ser.serialize_field("vmStatus", &self.vm_status)?;
        }
        if self.block_height != 0 {
            struct_ser.serialize_field("blockHeight", ToString::to_string(&self.block_height).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EventWithMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "event",
            "timestamp",
            "version",
            "hash",
            "success",
            "vm_status",
            "vmStatus",
            "block_height",
            "blockHeight",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Event,
            Timestamp,
            Version,
            Hash,
            Success,
            VmStatus,
            BlockHeight,
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
                            "event" => Ok(GeneratedField::Event),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "version" => Ok(GeneratedField::Version),
                            "hash" => Ok(GeneratedField::Hash),
                            "success" => Ok(GeneratedField::Success),
                            "vmStatus" | "vm_status" => Ok(GeneratedField::VmStatus),
                            "blockHeight" | "block_height" => Ok(GeneratedField::BlockHeight),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EventWithMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.EventWithMetadata")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventWithMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut event__ = None;
                let mut timestamp__ = None;
                let mut version__ = None;
                let mut hash__ = None;
                let mut success__ = None;
                let mut vm_status__ = None;
                let mut block_height__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Event => {
                            if event__.is_some() {
                                return Err(serde::de::Error::duplicate_field("event"));
                            }
                            event__ = map.next_value()?;
                        }
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
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = 
                                Some(map.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
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
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ = 
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(EventWithMetadata {
                    event: event__,
                    timestamp: timestamp__,
                    version: version__.unwrap_or_default(),
                    hash: hash__.unwrap_or_default(),
                    success: success__.unwrap_or_default(),
                    vm_status: vm_status__.unwrap_or_default(),
                    block_height: block_height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.EventWithMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EventsResponse {
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
        if self.chain_id.is_some() {
            len += 1;
        }
        if self.processed_range.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.EventsResponse", len)?;
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        if let Some(v) = self.chain_id.as_ref() {
            struct_ser.serialize_field("chainId", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.processed_range.as_ref() {
            struct_ser.serialize_field("processedRange", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EventsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "events",
            "chain_id",
            "chainId",
            "processed_range",
            "processedRange",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Events,
            ChainId,
            ProcessedRange,
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
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
                            "processedRange" | "processed_range" => Ok(GeneratedField::ProcessedRange),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EventsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.EventsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut events__ = None;
                let mut chain_id__ = None;
                let mut processed_range__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map.next_value()?);
                        }
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::ProcessedRange => {
                            if processed_range__.is_some() {
                                return Err(serde::de::Error::duplicate_field("processedRange"));
                            }
                            processed_range__ = map.next_value()?;
                        }
                    }
                }
                Ok(EventsResponse {
                    events: events__.unwrap_or_default(),
                    chain_id: chain_id__,
                    processed_range: processed_range__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.EventsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FullnodeInfo {
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
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.known_latest_version.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.FullnodeInfo", len)?;
        if self.chain_id != 0 {
            struct_ser.serialize_field("chainId", ToString::to_string(&self.chain_id).as_str())?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if let Some(v) = self.known_latest_version.as_ref() {
            struct_ser.serialize_field("knownLatestVersion", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FullnodeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_id",
            "chainId",
            "timestamp",
            "known_latest_version",
            "knownLatestVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainId,
            Timestamp,
            KnownLatestVersion,
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
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "knownLatestVersion" | "known_latest_version" => Ok(GeneratedField::KnownLatestVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FullnodeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.FullnodeInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<FullnodeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_id__ = None;
                let mut timestamp__ = None;
                let mut known_latest_version__ = None;
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
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map.next_value()?;
                        }
                        GeneratedField::KnownLatestVersion => {
                            if known_latest_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("knownLatestVersion"));
                            }
                            known_latest_version__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(FullnodeInfo {
                    chain_id: chain_id__.unwrap_or_default(),
                    timestamp: timestamp__,
                    known_latest_version: known_latest_version__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.FullnodeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetDataServiceForRequestRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.user_request.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.GetDataServiceForRequestRequest", len)?;
        if let Some(v) = self.user_request.as_ref() {
            struct_ser.serialize_field("userRequest", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetDataServiceForRequestRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "user_request",
            "userRequest",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            UserRequest,
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
                            "userRequest" | "user_request" => Ok(GeneratedField::UserRequest),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetDataServiceForRequestRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.GetDataServiceForRequestRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetDataServiceForRequestRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut user_request__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::UserRequest => {
                            if user_request__.is_some() {
                                return Err(serde::de::Error::duplicate_field("userRequest"));
                            }
                            user_request__ = map.next_value()?;
                        }
                    }
                }
                Ok(GetDataServiceForRequestRequest {
                    user_request: user_request__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.GetDataServiceForRequestRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetDataServiceForRequestResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.data_service_address.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.GetDataServiceForRequestResponse", len)?;
        if !self.data_service_address.is_empty() {
            struct_ser.serialize_field("dataServiceAddress", &self.data_service_address)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetDataServiceForRequestResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "data_service_address",
            "dataServiceAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DataServiceAddress,
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
                            "dataServiceAddress" | "data_service_address" => Ok(GeneratedField::DataServiceAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetDataServiceForRequestResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.GetDataServiceForRequestResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetDataServiceForRequestResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data_service_address__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::DataServiceAddress => {
                            if data_service_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dataServiceAddress"));
                            }
                            data_service_address__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(GetDataServiceForRequestResponse {
                    data_service_address: data_service_address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.GetDataServiceForRequestResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetEventsRequest {
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
        if self.batch_size.is_some() {
            len += 1;
        }
        if self.transaction_filter.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.GetEventsRequest", len)?;
        if let Some(v) = self.starting_version.as_ref() {
            struct_ser.serialize_field("startingVersion", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.transactions_count.as_ref() {
            struct_ser.serialize_field("transactionsCount", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.batch_size.as_ref() {
            struct_ser.serialize_field("batchSize", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.transaction_filter.as_ref() {
            struct_ser.serialize_field("transactionFilter", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetEventsRequest {
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
            "batch_size",
            "batchSize",
            "transaction_filter",
            "transactionFilter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartingVersion,
            TransactionsCount,
            BatchSize,
            TransactionFilter,
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
                            "batchSize" | "batch_size" => Ok(GeneratedField::BatchSize),
                            "transactionFilter" | "transaction_filter" => Ok(GeneratedField::TransactionFilter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetEventsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.GetEventsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetEventsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut starting_version__ = None;
                let mut transactions_count__ = None;
                let mut batch_size__ = None;
                let mut transaction_filter__ = None;
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
                        GeneratedField::BatchSize => {
                            if batch_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("batchSize"));
                            }
                            batch_size__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::TransactionFilter => {
                            if transaction_filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionFilter"));
                            }
                            transaction_filter__ = map.next_value()?;
                        }
                    }
                }
                Ok(GetEventsRequest {
                    starting_version: starting_version__,
                    transactions_count: transactions_count__,
                    batch_size: batch_size__,
                    transaction_filter: transaction_filter__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.GetEventsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTransactionsRequest {
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
        if self.batch_size.is_some() {
            len += 1;
        }
        if self.transaction_filter.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.GetTransactionsRequest", len)?;
        if let Some(v) = self.starting_version.as_ref() {
            struct_ser.serialize_field("startingVersion", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.transactions_count.as_ref() {
            struct_ser.serialize_field("transactionsCount", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.batch_size.as_ref() {
            struct_ser.serialize_field("batchSize", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.transaction_filter.as_ref() {
            struct_ser.serialize_field("transactionFilter", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionsRequest {
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
            "batch_size",
            "batchSize",
            "transaction_filter",
            "transactionFilter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartingVersion,
            TransactionsCount,
            BatchSize,
            TransactionFilter,
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
                            "batchSize" | "batch_size" => Ok(GeneratedField::BatchSize),
                            "transactionFilter" | "transaction_filter" => Ok(GeneratedField::TransactionFilter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.GetTransactionsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetTransactionsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut starting_version__ = None;
                let mut transactions_count__ = None;
                let mut batch_size__ = None;
                let mut transaction_filter__ = None;
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
                        GeneratedField::BatchSize => {
                            if batch_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("batchSize"));
                            }
                            batch_size__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::TransactionFilter => {
                            if transaction_filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionFilter"));
                            }
                            transaction_filter__ = map.next_value()?;
                        }
                    }
                }
                Ok(GetTransactionsRequest {
                    starting_version: starting_version__,
                    transactions_count: transactions_count__,
                    batch_size: batch_size__,
                    transaction_filter: transaction_filter__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.GetTransactionsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GrpcManagerInfo {
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
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.known_latest_version.is_some() {
            len += 1;
        }
        if self.master_address.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.GrpcManagerInfo", len)?;
        if self.chain_id != 0 {
            struct_ser.serialize_field("chainId", ToString::to_string(&self.chain_id).as_str())?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if let Some(v) = self.known_latest_version.as_ref() {
            struct_ser.serialize_field("knownLatestVersion", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.master_address.as_ref() {
            struct_ser.serialize_field("masterAddress", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GrpcManagerInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_id",
            "chainId",
            "timestamp",
            "known_latest_version",
            "knownLatestVersion",
            "master_address",
            "masterAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainId,
            Timestamp,
            KnownLatestVersion,
            MasterAddress,
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
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "knownLatestVersion" | "known_latest_version" => Ok(GeneratedField::KnownLatestVersion),
                            "masterAddress" | "master_address" => Ok(GeneratedField::MasterAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GrpcManagerInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.GrpcManagerInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GrpcManagerInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_id__ = None;
                let mut timestamp__ = None;
                let mut known_latest_version__ = None;
                let mut master_address__ = None;
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
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map.next_value()?;
                        }
                        GeneratedField::KnownLatestVersion => {
                            if known_latest_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("knownLatestVersion"));
                            }
                            known_latest_version__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::MasterAddress => {
                            if master_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("masterAddress"));
                            }
                            master_address__ = map.next_value()?;
                        }
                    }
                }
                Ok(GrpcManagerInfo {
                    chain_id: chain_id__.unwrap_or_default(),
                    timestamp: timestamp__,
                    known_latest_version: known_latest_version__,
                    master_address: master_address__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.GrpcManagerInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HeartbeatRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.service_info.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.HeartbeatRequest", len)?;
        if let Some(v) = self.service_info.as_ref() {
            struct_ser.serialize_field("serviceInfo", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeartbeatRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "service_info",
            "serviceInfo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ServiceInfo,
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
                            "serviceInfo" | "service_info" => Ok(GeneratedField::ServiceInfo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeartbeatRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.HeartbeatRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<HeartbeatRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut service_info__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ServiceInfo => {
                            if service_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("serviceInfo"));
                            }
                            service_info__ = map.next_value()?;
                        }
                    }
                }
                Ok(HeartbeatRequest {
                    service_info: service_info__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.HeartbeatRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HeartbeatResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.known_latest_version.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.HeartbeatResponse", len)?;
        if let Some(v) = self.known_latest_version.as_ref() {
            struct_ser.serialize_field("knownLatestVersion", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HeartbeatResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "known_latest_version",
            "knownLatestVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            KnownLatestVersion,
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
                            "knownLatestVersion" | "known_latest_version" => Ok(GeneratedField::KnownLatestVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HeartbeatResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.HeartbeatResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<HeartbeatResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut known_latest_version__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::KnownLatestVersion => {
                            if known_latest_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("knownLatestVersion"));
                            }
                            known_latest_version__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(HeartbeatResponse {
                    known_latest_version: known_latest_version__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.HeartbeatResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HistoricalDataServiceInfo {
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
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.known_latest_version.is_some() {
            len += 1;
        }
        if self.stream_info.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.HistoricalDataServiceInfo", len)?;
        if self.chain_id != 0 {
            struct_ser.serialize_field("chainId", ToString::to_string(&self.chain_id).as_str())?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if let Some(v) = self.known_latest_version.as_ref() {
            struct_ser.serialize_field("knownLatestVersion", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.stream_info.as_ref() {
            struct_ser.serialize_field("streamInfo", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HistoricalDataServiceInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_id",
            "chainId",
            "timestamp",
            "known_latest_version",
            "knownLatestVersion",
            "stream_info",
            "streamInfo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainId,
            Timestamp,
            KnownLatestVersion,
            StreamInfo,
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
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "knownLatestVersion" | "known_latest_version" => Ok(GeneratedField::KnownLatestVersion),
                            "streamInfo" | "stream_info" => Ok(GeneratedField::StreamInfo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HistoricalDataServiceInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.HistoricalDataServiceInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<HistoricalDataServiceInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_id__ = None;
                let mut timestamp__ = None;
                let mut known_latest_version__ = None;
                let mut stream_info__ = None;
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
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map.next_value()?;
                        }
                        GeneratedField::KnownLatestVersion => {
                            if known_latest_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("knownLatestVersion"));
                            }
                            known_latest_version__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::StreamInfo => {
                            if stream_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("streamInfo"));
                            }
                            stream_info__ = map.next_value()?;
                        }
                    }
                }
                Ok(HistoricalDataServiceInfo {
                    chain_id: chain_id__.unwrap_or_default(),
                    timestamp: timestamp__,
                    known_latest_version: known_latest_version__,
                    stream_info: stream_info__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.HistoricalDataServiceInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LiveDataServiceInfo {
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
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.known_latest_version.is_some() {
            len += 1;
        }
        if self.stream_info.is_some() {
            len += 1;
        }
        if self.min_servable_version.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.LiveDataServiceInfo", len)?;
        if self.chain_id != 0 {
            struct_ser.serialize_field("chainId", ToString::to_string(&self.chain_id).as_str())?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if let Some(v) = self.known_latest_version.as_ref() {
            struct_ser.serialize_field("knownLatestVersion", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.stream_info.as_ref() {
            struct_ser.serialize_field("streamInfo", v)?;
        }
        if let Some(v) = self.min_servable_version.as_ref() {
            struct_ser.serialize_field("minServableVersion", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LiveDataServiceInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_id",
            "chainId",
            "timestamp",
            "known_latest_version",
            "knownLatestVersion",
            "stream_info",
            "streamInfo",
            "min_servable_version",
            "minServableVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainId,
            Timestamp,
            KnownLatestVersion,
            StreamInfo,
            MinServableVersion,
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
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "knownLatestVersion" | "known_latest_version" => Ok(GeneratedField::KnownLatestVersion),
                            "streamInfo" | "stream_info" => Ok(GeneratedField::StreamInfo),
                            "minServableVersion" | "min_servable_version" => Ok(GeneratedField::MinServableVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LiveDataServiceInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.LiveDataServiceInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<LiveDataServiceInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_id__ = None;
                let mut timestamp__ = None;
                let mut known_latest_version__ = None;
                let mut stream_info__ = None;
                let mut min_servable_version__ = None;
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
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map.next_value()?;
                        }
                        GeneratedField::KnownLatestVersion => {
                            if known_latest_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("knownLatestVersion"));
                            }
                            known_latest_version__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::StreamInfo => {
                            if stream_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("streamInfo"));
                            }
                            stream_info__ = map.next_value()?;
                        }
                        GeneratedField::MinServableVersion => {
                            if min_servable_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minServableVersion"));
                            }
                            min_servable_version__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(LiveDataServiceInfo {
                    chain_id: chain_id__.unwrap_or_default(),
                    timestamp: timestamp__,
                    known_latest_version: known_latest_version__,
                    stream_info: stream_info__,
                    min_servable_version: min_servable_version__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.LiveDataServiceInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LogicalAndFilters {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.filters.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.LogicalAndFilters", len)?;
        if !self.filters.is_empty() {
            struct_ser.serialize_field("filters", &self.filters)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LogicalAndFilters {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "filters",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Filters,
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
                            "filters" => Ok(GeneratedField::Filters),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LogicalAndFilters;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.LogicalAndFilters")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<LogicalAndFilters, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut filters__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Filters => {
                            if filters__.is_some() {
                                return Err(serde::de::Error::duplicate_field("filters"));
                            }
                            filters__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(LogicalAndFilters {
                    filters: filters__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.LogicalAndFilters", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LogicalOrFilters {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.filters.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.LogicalOrFilters", len)?;
        if !self.filters.is_empty() {
            struct_ser.serialize_field("filters", &self.filters)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LogicalOrFilters {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "filters",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Filters,
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
                            "filters" => Ok(GeneratedField::Filters),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LogicalOrFilters;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.LogicalOrFilters")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<LogicalOrFilters, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut filters__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Filters => {
                            if filters__.is_some() {
                                return Err(serde::de::Error::duplicate_field("filters"));
                            }
                            filters__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(LogicalOrFilters {
                    filters: filters__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.LogicalOrFilters", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MoveStructTagFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.address.is_some() {
            len += 1;
        }
        if self.module.is_some() {
            len += 1;
        }
        if self.name.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.MoveStructTagFilter", len)?;
        if let Some(v) = self.address.as_ref() {
            struct_ser.serialize_field("address", v)?;
        }
        if let Some(v) = self.module.as_ref() {
            struct_ser.serialize_field("module", v)?;
        }
        if let Some(v) = self.name.as_ref() {
            struct_ser.serialize_field("name", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveStructTagFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "module",
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
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
                            "address" => Ok(GeneratedField::Address),
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
            type Value = MoveStructTagFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.MoveStructTagFilter")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<MoveStructTagFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut module__ = None;
                let mut name__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = map.next_value()?;
                        }
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
                            name__ = map.next_value()?;
                        }
                    }
                }
                Ok(MoveStructTagFilter {
                    address: address__,
                    module: module__,
                    name: name__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.MoveStructTagFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PingDataServiceRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.known_latest_version.is_some() {
            len += 1;
        }
        if self.ping_live_data_service {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.PingDataServiceRequest", len)?;
        if let Some(v) = self.known_latest_version.as_ref() {
            struct_ser.serialize_field("knownLatestVersion", ToString::to_string(&v).as_str())?;
        }
        if self.ping_live_data_service {
            struct_ser.serialize_field("pingLiveDataService", &self.ping_live_data_service)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PingDataServiceRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "known_latest_version",
            "knownLatestVersion",
            "ping_live_data_service",
            "pingLiveDataService",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            KnownLatestVersion,
            PingLiveDataService,
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
                            "knownLatestVersion" | "known_latest_version" => Ok(GeneratedField::KnownLatestVersion),
                            "pingLiveDataService" | "ping_live_data_service" => Ok(GeneratedField::PingLiveDataService),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PingDataServiceRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.PingDataServiceRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PingDataServiceRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut known_latest_version__ = None;
                let mut ping_live_data_service__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::KnownLatestVersion => {
                            if known_latest_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("knownLatestVersion"));
                            }
                            known_latest_version__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::PingLiveDataService => {
                            if ping_live_data_service__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pingLiveDataService"));
                            }
                            ping_live_data_service__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(PingDataServiceRequest {
                    known_latest_version: known_latest_version__,
                    ping_live_data_service: ping_live_data_service__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.PingDataServiceRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PingDataServiceResponse {
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
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.PingDataServiceResponse", len)?;
        if let Some(v) = self.info.as_ref() {
            match v {
                ping_data_service_response::Info::LiveDataServiceInfo(v) => {
                    struct_ser.serialize_field("liveDataServiceInfo", v)?;
                }
                ping_data_service_response::Info::HistoricalDataServiceInfo(v) => {
                    struct_ser.serialize_field("historicalDataServiceInfo", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PingDataServiceResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "live_data_service_info",
            "liveDataServiceInfo",
            "historical_data_service_info",
            "historicalDataServiceInfo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            LiveDataServiceInfo,
            HistoricalDataServiceInfo,
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
                            "liveDataServiceInfo" | "live_data_service_info" => Ok(GeneratedField::LiveDataServiceInfo),
                            "historicalDataServiceInfo" | "historical_data_service_info" => Ok(GeneratedField::HistoricalDataServiceInfo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PingDataServiceResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.PingDataServiceResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<PingDataServiceResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut info__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::LiveDataServiceInfo => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("liveDataServiceInfo"));
                            }
                            info__ = map.next_value::<::std::option::Option<_>>()?.map(ping_data_service_response::Info::LiveDataServiceInfo)
;
                        }
                        GeneratedField::HistoricalDataServiceInfo => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("historicalDataServiceInfo"));
                            }
                            info__ = map.next_value::<::std::option::Option<_>>()?.map(ping_data_service_response::Info::HistoricalDataServiceInfo)
;
                        }
                    }
                }
                Ok(PingDataServiceResponse {
                    info: info__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.PingDataServiceResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ProcessedRange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.first_version != 0 {
            len += 1;
        }
        if self.last_version != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.ProcessedRange", len)?;
        if self.first_version != 0 {
            struct_ser.serialize_field("firstVersion", ToString::to_string(&self.first_version).as_str())?;
        }
        if self.last_version != 0 {
            struct_ser.serialize_field("lastVersion", ToString::to_string(&self.last_version).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ProcessedRange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "first_version",
            "firstVersion",
            "last_version",
            "lastVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            FirstVersion,
            LastVersion,
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
                            "firstVersion" | "first_version" => Ok(GeneratedField::FirstVersion),
                            "lastVersion" | "last_version" => Ok(GeneratedField::LastVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ProcessedRange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.ProcessedRange")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ProcessedRange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut first_version__ = None;
                let mut last_version__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::FirstVersion => {
                            if first_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("firstVersion"));
                            }
                            first_version__ = 
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::LastVersion => {
                            if last_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lastVersion"));
                            }
                            last_version__ = 
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(ProcessedRange {
                    first_version: first_version__.unwrap_or_default(),
                    last_version: last_version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.ProcessedRange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ServiceInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.address.is_some() {
            len += 1;
        }
        if self.info.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.ServiceInfo", len)?;
        if let Some(v) = self.address.as_ref() {
            struct_ser.serialize_field("address", v)?;
        }
        if let Some(v) = self.info.as_ref() {
            match v {
                service_info::Info::LiveDataServiceInfo(v) => {
                    struct_ser.serialize_field("liveDataServiceInfo", v)?;
                }
                service_info::Info::HistoricalDataServiceInfo(v) => {
                    struct_ser.serialize_field("historicalDataServiceInfo", v)?;
                }
                service_info::Info::FullnodeInfo(v) => {
                    struct_ser.serialize_field("fullnodeInfo", v)?;
                }
                service_info::Info::GrpcManagerInfo(v) => {
                    struct_ser.serialize_field("grpcManagerInfo", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ServiceInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "live_data_service_info",
            "liveDataServiceInfo",
            "historical_data_service_info",
            "historicalDataServiceInfo",
            "fullnode_info",
            "fullnodeInfo",
            "grpc_manager_info",
            "grpcManagerInfo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            LiveDataServiceInfo,
            HistoricalDataServiceInfo,
            FullnodeInfo,
            GrpcManagerInfo,
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
                            "liveDataServiceInfo" | "live_data_service_info" => Ok(GeneratedField::LiveDataServiceInfo),
                            "historicalDataServiceInfo" | "historical_data_service_info" => Ok(GeneratedField::HistoricalDataServiceInfo),
                            "fullnodeInfo" | "fullnode_info" => Ok(GeneratedField::FullnodeInfo),
                            "grpcManagerInfo" | "grpc_manager_info" => Ok(GeneratedField::GrpcManagerInfo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ServiceInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.ServiceInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<ServiceInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut info__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = map.next_value()?;
                        }
                        GeneratedField::LiveDataServiceInfo => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("liveDataServiceInfo"));
                            }
                            info__ = map.next_value::<::std::option::Option<_>>()?.map(service_info::Info::LiveDataServiceInfo)
;
                        }
                        GeneratedField::HistoricalDataServiceInfo => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("historicalDataServiceInfo"));
                            }
                            info__ = map.next_value::<::std::option::Option<_>>()?.map(service_info::Info::HistoricalDataServiceInfo)
;
                        }
                        GeneratedField::FullnodeInfo => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fullnodeInfo"));
                            }
                            info__ = map.next_value::<::std::option::Option<_>>()?.map(service_info::Info::FullnodeInfo)
;
                        }
                        GeneratedField::GrpcManagerInfo => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("grpcManagerInfo"));
                            }
                            info__ = map.next_value::<::std::option::Option<_>>()?.map(service_info::Info::GrpcManagerInfo)
;
                        }
                    }
                }
                Ok(ServiceInfo {
                    address: address__,
                    info: info__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.ServiceInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.active_streams.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.StreamInfo", len)?;
        if !self.active_streams.is_empty() {
            struct_ser.serialize_field("activeStreams", &self.active_streams)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "active_streams",
            "activeStreams",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ActiveStreams,
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
                            "activeStreams" | "active_streams" => Ok(GeneratedField::ActiveStreams),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StreamInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.StreamInfo")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<StreamInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut active_streams__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::ActiveStreams => {
                            if active_streams__.is_some() {
                                return Err(serde::de::Error::duplicate_field("activeStreams"));
                            }
                            active_streams__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(StreamInfo {
                    active_streams: active_streams__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.StreamInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamProgress {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.samples.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.StreamProgress", len)?;
        if !self.samples.is_empty() {
            struct_ser.serialize_field("samples", &self.samples)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamProgress {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "samples",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Samples,
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
                            "samples" => Ok(GeneratedField::Samples),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StreamProgress;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.StreamProgress")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<StreamProgress, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut samples__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Samples => {
                            if samples__.is_some() {
                                return Err(serde::de::Error::duplicate_field("samples"));
                            }
                            samples__ = Some(map.next_value()?);
                        }
                    }
                }
                Ok(StreamProgress {
                    samples: samples__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.StreamProgress", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamProgressSampleProto {
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
        if self.size_bytes != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.StreamProgressSampleProto", len)?;
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if self.version != 0 {
            struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        }
        if self.size_bytes != 0 {
            struct_ser.serialize_field("sizeBytes", ToString::to_string(&self.size_bytes).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamProgressSampleProto {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "timestamp",
            "version",
            "size_bytes",
            "sizeBytes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Timestamp,
            Version,
            SizeBytes,
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
                            "sizeBytes" | "size_bytes" => Ok(GeneratedField::SizeBytes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StreamProgressSampleProto;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.StreamProgressSampleProto")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<StreamProgressSampleProto, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut timestamp__ = None;
                let mut version__ = None;
                let mut size_bytes__ = None;
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
                        GeneratedField::SizeBytes => {
                            if size_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sizeBytes"));
                            }
                            size_bytes__ = 
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(StreamProgressSampleProto {
                    timestamp: timestamp__,
                    version: version__.unwrap_or_default(),
                    size_bytes: size_bytes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.StreamProgressSampleProto", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionRootFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.success.is_some() {
            len += 1;
        }
        if self.transaction_type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.TransactionRootFilter", len)?;
        if let Some(v) = self.success.as_ref() {
            struct_ser.serialize_field("success", v)?;
        }
        if let Some(v) = self.transaction_type.as_ref() {
            let v = super::super::transaction::v1::transaction::TransactionType::from_i32(*v)
                .ok_or_else(|| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("transactionType", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionRootFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "success",
            "transaction_type",
            "transactionType",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Success,
            TransactionType,
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
                            "success" => Ok(GeneratedField::Success),
                            "transactionType" | "transaction_type" => Ok(GeneratedField::TransactionType),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionRootFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.TransactionRootFilter")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionRootFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut success__ = None;
                let mut transaction_type__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Success => {
                            if success__.is_some() {
                                return Err(serde::de::Error::duplicate_field("success"));
                            }
                            success__ = map.next_value()?;
                        }
                        GeneratedField::TransactionType => {
                            if transaction_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionType"));
                            }
                            transaction_type__ = map.next_value::<::std::option::Option<super::super::transaction::v1::transaction::TransactionType>>()?.map(|x| x as i32);
                        }
                    }
                }
                Ok(TransactionRootFilter {
                    success: success__,
                    transaction_type: transaction_type__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.TransactionRootFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionsInStorage {
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
        if self.starting_version.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.TransactionsInStorage", len)?;
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        if let Some(v) = self.starting_version.as_ref() {
            struct_ser.serialize_field("startingVersion", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionsInStorage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transactions",
            "starting_version",
            "startingVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transactions,
            StartingVersion,
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
                            "startingVersion" | "starting_version" => Ok(GeneratedField::StartingVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionsInStorage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.TransactionsInStorage")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionsInStorage, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transactions__ = None;
                let mut starting_version__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ = Some(map.next_value()?);
                        }
                        GeneratedField::StartingVersion => {
                            if starting_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startingVersion"));
                            }
                            starting_version__ = 
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(TransactionsInStorage {
                    transactions: transactions__.unwrap_or_default(),
                    starting_version: starting_version__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.TransactionsInStorage", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionsResponse {
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
        if self.chain_id.is_some() {
            len += 1;
        }
        if self.processed_range.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.TransactionsResponse", len)?;
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        if let Some(v) = self.chain_id.as_ref() {
            struct_ser.serialize_field("chainId", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.processed_range.as_ref() {
            struct_ser.serialize_field("processedRange", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transactions",
            "chain_id",
            "chainId",
            "processed_range",
            "processedRange",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transactions,
            ChainId,
            ProcessedRange,
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
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
                            "processedRange" | "processed_range" => Ok(GeneratedField::ProcessedRange),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.TransactionsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<TransactionsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transactions__ = None;
                let mut chain_id__ = None;
                let mut processed_range__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
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
                                map.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::ProcessedRange => {
                            if processed_range__.is_some() {
                                return Err(serde::de::Error::duplicate_field("processedRange"));
                            }
                            processed_range__ = map.next_value()?;
                        }
                    }
                }
                Ok(TransactionsResponse {
                    transactions: transactions__.unwrap_or_default(),
                    chain_id: chain_id__,
                    processed_range: processed_range__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.TransactionsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UserTransactionFilter {
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
        if self.payload_filter.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.UserTransactionFilter", len)?;
        if let Some(v) = self.sender.as_ref() {
            struct_ser.serialize_field("sender", v)?;
        }
        if let Some(v) = self.payload_filter.as_ref() {
            struct_ser.serialize_field("payloadFilter", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UserTransactionFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sender",
            "payload_filter",
            "payloadFilter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Sender,
            PayloadFilter,
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
                            "payloadFilter" | "payload_filter" => Ok(GeneratedField::PayloadFilter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UserTransactionFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.UserTransactionFilter")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<UserTransactionFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sender__ = None;
                let mut payload_filter__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::Sender => {
                            if sender__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sender"));
                            }
                            sender__ = map.next_value()?;
                        }
                        GeneratedField::PayloadFilter => {
                            if payload_filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payloadFilter"));
                            }
                            payload_filter__ = map.next_value()?;
                        }
                    }
                }
                Ok(UserTransactionFilter {
                    sender: sender__,
                    payload_filter: payload_filter__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.UserTransactionFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UserTransactionPayloadFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.entry_function_filter.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("aptos.indexer.v1.UserTransactionPayloadFilter", len)?;
        if let Some(v) = self.entry_function_filter.as_ref() {
            struct_ser.serialize_field("entryFunctionFilter", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UserTransactionPayloadFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "entry_function_filter",
            "entryFunctionFilter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            EntryFunctionFilter,
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
                            "entryFunctionFilter" | "entry_function_filter" => Ok(GeneratedField::EntryFunctionFilter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UserTransactionPayloadFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.indexer.v1.UserTransactionPayloadFilter")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<UserTransactionPayloadFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut entry_function_filter__ = None;
                while let Some(k) = map.next_key()? {
                    match k {
                        GeneratedField::EntryFunctionFilter => {
                            if entry_function_filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entryFunctionFilter"));
                            }
                            entry_function_filter__ = map.next_value()?;
                        }
                    }
                }
                Ok(UserTransactionPayloadFilter {
                    entry_function_filter: entry_function_filter__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.indexer.v1.UserTransactionPayloadFilter", FIELDS, GeneratedVisitor)
    }
}
