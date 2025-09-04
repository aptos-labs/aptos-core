// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// @generated
impl serde::Serialize for Transaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 14;
        if self.payload.is_some() {
            len += 1;
        }
        if self.state_checkpoint_hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("velor.bigquery_schema.transaction.v1.Transaction", len)?;
        #[allow(clippy::needless_borrow)]
        struct_ser.serialize_field("version", ToString::to_string(&self.version).as_str())?;
        #[allow(clippy::needless_borrow)]
        struct_ser.serialize_field("blockHeight", ToString::to_string(&self.block_height).as_str())?;
        struct_ser.serialize_field("hash", &self.hash)?;
        struct_ser.serialize_field("type", &self.r#type)?;
        if let Some(v) = self.payload.as_ref() {
            struct_ser.serialize_field("payload", v)?;
        }
        struct_ser.serialize_field("stateChangeHash", &self.state_change_hash)?;
        struct_ser.serialize_field("eventRootHash", &self.event_root_hash)?;
        if let Some(v) = self.state_checkpoint_hash.as_ref() {
            struct_ser.serialize_field("stateCheckpointHash", v)?;
        }
        #[allow(clippy::needless_borrow)]
        struct_ser.serialize_field("gasUsed", ToString::to_string(&self.gas_used).as_str())?;
        struct_ser.serialize_field("success", &self.success)?;
        struct_ser.serialize_field("vmStatus", &self.vm_status)?;
        struct_ser.serialize_field("accumulatorRootHash", &self.accumulator_root_hash)?;
        #[allow(clippy::needless_borrow)]
        struct_ser.serialize_field("numEvents", ToString::to_string(&self.num_events).as_str())?;
        #[allow(clippy::needless_borrow)]
        struct_ser.serialize_field("numWriteSetChanges", ToString::to_string(&self.num_write_set_changes).as_str())?;
        #[allow(clippy::needless_borrow)]
        struct_ser.serialize_field("epoch", ToString::to_string(&self.epoch).as_str())?;
        #[allow(clippy::needless_borrow)]
        struct_ser.serialize_field("insertedAt", ToString::to_string(&self.inserted_at).as_str())?;
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
            "version",
            "block_height",
            "blockHeight",
            "hash",
            "type",
            "payload",
            "state_change_hash",
            "stateChangeHash",
            "event_root_hash",
            "eventRootHash",
            "state_checkpoint_hash",
            "stateCheckpointHash",
            "gas_used",
            "gasUsed",
            "success",
            "vm_status",
            "vmStatus",
            "accumulator_root_hash",
            "accumulatorRootHash",
            "num_events",
            "numEvents",
            "num_write_set_changes",
            "numWriteSetChanges",
            "epoch",
            "inserted_at",
            "insertedAt",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Version,
            BlockHeight,
            Hash,
            Type,
            Payload,
            StateChangeHash,
            EventRootHash,
            StateCheckpointHash,
            GasUsed,
            Success,
            VmStatus,
            AccumulatorRootHash,
            NumEvents,
            NumWriteSetChanges,
            Epoch,
            InsertedAt,
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
                            "version" => Ok(GeneratedField::Version),
                            "blockHeight" | "block_height" => Ok(GeneratedField::BlockHeight),
                            "hash" => Ok(GeneratedField::Hash),
                            "type" => Ok(GeneratedField::Type),
                            "payload" => Ok(GeneratedField::Payload),
                            "stateChangeHash" | "state_change_hash" => Ok(GeneratedField::StateChangeHash),
                            "eventRootHash" | "event_root_hash" => Ok(GeneratedField::EventRootHash),
                            "stateCheckpointHash" | "state_checkpoint_hash" => Ok(GeneratedField::StateCheckpointHash),
                            "gasUsed" | "gas_used" => Ok(GeneratedField::GasUsed),
                            "success" => Ok(GeneratedField::Success),
                            "vmStatus" | "vm_status" => Ok(GeneratedField::VmStatus),
                            "accumulatorRootHash" | "accumulator_root_hash" => Ok(GeneratedField::AccumulatorRootHash),
                            "numEvents" | "num_events" => Ok(GeneratedField::NumEvents),
                            "numWriteSetChanges" | "num_write_set_changes" => Ok(GeneratedField::NumWriteSetChanges),
                            "epoch" => Ok(GeneratedField::Epoch),
                            "insertedAt" | "inserted_at" => Ok(GeneratedField::InsertedAt),
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
                formatter.write_str("struct velor.bigquery_schema.transaction.v1.Transaction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Transaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut version__ = None;
                let mut block_height__ = None;
                let mut hash__ = None;
                let mut r#type__ = None;
                let mut payload__ = None;
                let mut state_change_hash__ = None;
                let mut event_root_hash__ = None;
                let mut state_checkpoint_hash__ = None;
                let mut gas_used__ = None;
                let mut success__ = None;
                let mut vm_status__ = None;
                let mut accumulator_root_hash__ = None;
                let mut num_events__ = None;
                let mut num_write_set_changes__ = None;
                let mut epoch__ = None;
                let mut inserted_at__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ =
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ =
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Payload => {
                            if payload__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload__ = map_.next_value()?;
                        }
                        GeneratedField::StateChangeHash => {
                            if state_change_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateChangeHash"));
                            }
                            state_change_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::EventRootHash => {
                            if event_root_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("eventRootHash"));
                            }
                            event_root_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StateCheckpointHash => {
                            if state_checkpoint_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stateCheckpointHash"));
                            }
                            state_checkpoint_hash__ = map_.next_value()?;
                        }
                        GeneratedField::GasUsed => {
                            if gas_used__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gasUsed"));
                            }
                            gas_used__ =
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Success => {
                            if success__.is_some() {
                                return Err(serde::de::Error::duplicate_field("success"));
                            }
                            success__ = Some(map_.next_value()?);
                        }
                        GeneratedField::VmStatus => {
                            if vm_status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("vmStatus"));
                            }
                            vm_status__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AccumulatorRootHash => {
                            if accumulator_root_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("accumulatorRootHash"));
                            }
                            accumulator_root_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NumEvents => {
                            if num_events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numEvents"));
                            }
                            num_events__ =
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NumWriteSetChanges => {
                            if num_write_set_changes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numWriteSetChanges"));
                            }
                            num_write_set_changes__ =
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Epoch => {
                            if epoch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("epoch"));
                            }
                            epoch__ =
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::InsertedAt => {
                            if inserted_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("insertedAt"));
                            }
                            inserted_at__ =
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Transaction {
                    version: version__.ok_or_else(|| serde::de::Error::missing_field("version"))?,
                    block_height: block_height__.ok_or_else(|| serde::de::Error::missing_field("blockHeight"))?,
                    hash: hash__.ok_or_else(|| serde::de::Error::missing_field("hash"))?,
                    r#type: r#type__.ok_or_else(|| serde::de::Error::missing_field("type"))?,
                    payload: payload__,
                    state_change_hash: state_change_hash__.ok_or_else(|| serde::de::Error::missing_field("stateChangeHash"))?,
                    event_root_hash: event_root_hash__.ok_or_else(|| serde::de::Error::missing_field("eventRootHash"))?,
                    state_checkpoint_hash: state_checkpoint_hash__,
                    gas_used: gas_used__.ok_or_else(|| serde::de::Error::missing_field("gasUsed"))?,
                    success: success__.ok_or_else(|| serde::de::Error::missing_field("success"))?,
                    vm_status: vm_status__.ok_or_else(|| serde::de::Error::missing_field("vmStatus"))?,
                    accumulator_root_hash: accumulator_root_hash__.ok_or_else(|| serde::de::Error::missing_field("accumulatorRootHash"))?,
                    num_events: num_events__.ok_or_else(|| serde::de::Error::missing_field("numEvents"))?,
                    num_write_set_changes: num_write_set_changes__.ok_or_else(|| serde::de::Error::missing_field("numWriteSetChanges"))?,
                    epoch: epoch__.ok_or_else(|| serde::de::Error::missing_field("epoch"))?,
                    inserted_at: inserted_at__.ok_or_else(|| serde::de::Error::missing_field("insertedAt"))?,
                })
            }
        }
        deserializer.deserialize_struct("velor.bigquery_schema.transaction.v1.Transaction", FIELDS, GeneratedVisitor)
    }
}
