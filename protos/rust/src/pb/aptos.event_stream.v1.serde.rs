// @generated
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
        if self.transaction_version != 0 {
            len += 1;
        }
        if self.transaction_timestamp.is_some() {
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
        let mut struct_ser = serializer.serialize_struct("aptos.event_stream.v1.Event", len)?;
        if let Some(v) = self.key.as_ref() {
            struct_ser.serialize_field("key", v)?;
        }
        if self.sequence_number != 0 {
            struct_ser.serialize_field("sequenceNumber", ToString::to_string(&self.sequence_number).as_str())?;
        }
        if self.transaction_version != 0 {
            struct_ser.serialize_field("transactionVersion", ToString::to_string(&self.transaction_version).as_str())?;
        }
        if let Some(v) = self.transaction_timestamp.as_ref() {
            struct_ser.serialize_field("transactionTimestamp", v)?;
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
            "transaction_version",
            "transactionVersion",
            "transaction_timestamp",
            "transactionTimestamp",
            "type",
            "type_str",
            "typeStr",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            SequenceNumber,
            TransactionVersion,
            TransactionTimestamp,
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
                            "transactionVersion" | "transaction_version" => Ok(GeneratedField::TransactionVersion),
                            "transactionTimestamp" | "transaction_timestamp" => Ok(GeneratedField::TransactionTimestamp),
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
                formatter.write_str("struct aptos.event_stream.v1.Event")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<Event, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut sequence_number__ = None;
                let mut transaction_version__ = None;
                let mut transaction_timestamp__ = None;
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
                        GeneratedField::TransactionVersion => {
                            if transaction_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionVersion"));
                            }
                            transaction_version__ = 
                                Some(map.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TransactionTimestamp => {
                            if transaction_timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionTimestamp"));
                            }
                            transaction_timestamp__ = map.next_value()?;
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
                    transaction_version: transaction_version__.unwrap_or_default(),
                    transaction_timestamp: transaction_timestamp__,
                    r#type: r#type__,
                    type_str: type_str__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("aptos.event_stream.v1.Event", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EventsInStorage {
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
        let mut struct_ser = serializer.serialize_struct("aptos.event_stream.v1.EventsInStorage", len)?;
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        if let Some(v) = self.starting_version.as_ref() {
            struct_ser.serialize_field("startingVersion", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EventsInStorage {
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
            type Value = EventsInStorage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.event_stream.v1.EventsInStorage")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventsInStorage, V::Error>
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
                Ok(EventsInStorage {
                    transactions: transactions__.unwrap_or_default(),
                    starting_version: starting_version__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.event_stream.v1.EventsInStorage", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("aptos.event_stream.v1.EventsResponse", len)?;
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        if let Some(v) = self.chain_id.as_ref() {
            struct_ser.serialize_field("chainId", ToString::to_string(&v).as_str())?;
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
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Events,
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
                            "events" => Ok(GeneratedField::Events),
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
            type Value = EventsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct aptos.event_stream.v1.EventsResponse")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<EventsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut events__ = None;
                let mut chain_id__ = None;
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
                    }
                }
                Ok(EventsResponse {
                    events: events__.unwrap_or_default(),
                    chain_id: chain_id__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.event_stream.v1.EventsResponse", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("aptos.event_stream.v1.GetEventsRequest", len)?;
        if let Some(v) = self.starting_version.as_ref() {
            struct_ser.serialize_field("startingVersion", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.transactions_count.as_ref() {
            struct_ser.serialize_field("transactionsCount", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.batch_size.as_ref() {
            struct_ser.serialize_field("batchSize", ToString::to_string(&v).as_str())?;
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
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartingVersion,
            TransactionsCount,
            BatchSize,
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
                formatter.write_str("struct aptos.event_stream.v1.GetEventsRequest")
            }

            fn visit_map<V>(self, mut map: V) -> std::result::Result<GetEventsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut starting_version__ = None;
                let mut transactions_count__ = None;
                let mut batch_size__ = None;
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
                    }
                }
                Ok(GetEventsRequest {
                    starting_version: starting_version__,
                    transactions_count: transactions_count__,
                    batch_size: batch_size__,
                })
            }
        }
        deserializer.deserialize_struct("aptos.event_stream.v1.GetEventsRequest", FIELDS, GeneratedVisitor)
    }
}
