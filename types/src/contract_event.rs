// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::{
        DepositEvent, NewBlockEvent, NewEpochEvent, WithdrawEvent, NEW_EPOCH_EVENT_MOVE_TYPE_TAG,
        NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG,
    },
    dkg::DKGStartEvent,
    event::EventKey,
    jwks::{
        jwk::JWKMoveStruct, rsa::RSA_JWK, unsupported::UnsupportedJWK, AllProvidersJWKs,
        ObservedJWKsUpdated, ProviderJWKs,
    },
    move_any::{Any, AsMoveAny},
    transaction::Version,
    validator_verifier::ValidatorConsensusInfo,
};
use anyhow::{bail, Error, Result};
use api_types::events::contract_event::GravityEvent;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use hex;
use move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{convert::TryFrom, ops::Deref, str::FromStr};

/// Helper function to convert ValidatorConsensusInfoMoveStruct to ValidatorConsensusInfo
fn convert_validator_consensus_info(
    v: &api_types::on_chain_config::dkg::ValidatorConsensusInfo,
) -> Result<ValidatorConsensusInfo, Error> {
    let addr =
        crate::account_address::AccountAddress::from_bytes(&v.addr.bytes()).map_err(|e| {
            tracing::error!("Failed to parse address: {:?}, error: {}", v.addr, e);
            e
        })?;

    // pk_bytes is already raw bytes (Vec<u8>), just validate length
    if v.pk_bytes.len() != 48 {
        return Err(anyhow::anyhow!(
            "Invalid BLS12381 public key length: expected 48 bytes, got {} bytes",
            v.pk_bytes.len()
        ));
    }

    let public_key =
        aptos_crypto::bls12381::PublicKey::try_from(v.pk_bytes.as_slice()).map_err(|e| {
            tracing::error!(
                "Failed to parse BLS12381 public key: pk_bytes length: {}, bytes: {:?}, error: {}",
                v.pk_bytes.len(),
                v.pk_bytes,
                e
            );
            e
        })?;

    Ok(ValidatorConsensusInfo {
        address: addr,
        public_key,
        voting_power: v.voting_power,
    })
}

pub static FEE_STATEMENT_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("transaction_fee").to_owned(),
        name: ident_str!("FeeStatement").to_owned(),
        type_args: vec![],
    }))
});

/// This trait is used by block executor to abstractly represent an event,
/// and update its data.
pub trait TransactionEvent {
    fn get_event_data(&self) -> &[u8];
    fn set_event_data(&mut self, event_data: Vec<u8>);
}

/// Support versioning of the data structure.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ContractEvent {
    V1(ContractEventV1),
    V2(ContractEventV2),
}

impl TransactionEvent for ContractEvent {
    fn get_event_data(&self) -> &[u8] {
        match self {
            ContractEvent::V1(event) => event.event_data(),
            ContractEvent::V2(event) => event.event_data(),
        }
    }

    fn set_event_data(&mut self, event_data: Vec<u8>) {
        match self {
            ContractEvent::V1(event) => event.event_data = event_data,
            ContractEvent::V2(event) => event.event_data = event_data,
        }
    }
}

impl ContractEvent {
    pub fn new_v1(
        key: EventKey,
        sequence_number: u64,
        type_tag: TypeTag,
        event_data: Vec<u8>,
    ) -> Self {
        ContractEvent::V1(ContractEventV1::new(
            key,
            sequence_number,
            type_tag,
            event_data,
        ))
    }

    pub fn new_v2(type_tag: TypeTag, event_data: Vec<u8>) -> Self {
        ContractEvent::V2(ContractEventV2::new(type_tag, event_data))
    }

    pub fn new_v2_with_type_tag_str(type_tag_str: &str, event_data: Vec<u8>) -> Self {
        ContractEvent::V2(ContractEventV2::new(
            TypeTag::from_str(type_tag_str).unwrap(),
            event_data,
        ))
    }

    pub fn event_key(&self) -> Option<&EventKey> {
        match self {
            ContractEvent::V1(event) => Some(event.key()),
            ContractEvent::V2(_event) => None,
        }
    }

    pub fn event_data(&self) -> &[u8] {
        match self {
            ContractEvent::V1(event) => event.event_data(),
            ContractEvent::V2(event) => event.event_data(),
        }
    }

    pub fn type_tag(&self) -> &TypeTag {
        match self {
            ContractEvent::V1(event) => &event.type_tag,
            ContractEvent::V2(event) => &event.type_tag,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            ContractEvent::V1(event) => event.size(),
            ContractEvent::V2(event) => event.size(),
        }
    }

    pub fn is_v1(&self) -> bool {
        matches!(self, ContractEvent::V1(_))
    }

    pub fn is_v2(&self) -> bool {
        matches!(self, ContractEvent::V2(_))
    }

    pub fn v1(&self) -> Result<&ContractEventV1> {
        Ok(match self {
            ContractEvent::V1(event) => event,
            ContractEvent::V2(_event) => bail!("This is a module event"),
        })
    }

    pub fn v2(&self) -> Result<&ContractEventV2> {
        Ok(match self {
            ContractEvent::V1(_event) => bail!("This is a instance event"),
            ContractEvent::V2(event) => event,
        })
    }

    pub fn try_v2(&self) -> Option<&ContractEventV2> {
        match self {
            ContractEvent::V1(_event) => None,
            ContractEvent::V2(event) => Some(event),
        }
    }

    pub fn try_v2_typed<T: DeserializeOwned>(&self, event_type: &TypeTag) -> Result<Option<T>> {
        if let Some(v2) = self.try_v2() {
            if &v2.type_tag == event_type {
                return Ok(Some(bcs::from_bytes(&v2.event_data)?));
            }
        }

        Ok(None)
    }

    pub fn is_new_epoch_event(&self) -> bool {
        self.type_tag() == NEW_EPOCH_EVENT_MOVE_TYPE_TAG.deref()
            || self.type_tag() == NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG.deref()
    }

    pub fn expect_new_block_event(&self) -> Result<NewBlockEvent> {
        NewBlockEvent::try_from_bytes(self.event_data())
    }
}

/// Entry produced via a call to the `emit_event` builtin.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, CryptoHasher)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ContractEventV1 {
    /// The unique key that the event was emitted to
    key: EventKey,
    /// The number of messages that have been emitted to the path previously
    sequence_number: u64,
    /// The type of the data
    type_tag: TypeTag,
    /// The data payload of the event
    #[serde(with = "serde_bytes")]
    event_data: Vec<u8>,
}

impl ContractEventV1 {
    pub fn new(
        key: EventKey,
        sequence_number: u64,
        type_tag: TypeTag,
        event_data: Vec<u8>,
    ) -> Self {
        Self {
            key,
            sequence_number,
            type_tag,
            event_data,
        }
    }

    pub fn key(&self) -> &EventKey {
        &self.key
    }

    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn event_data(&self) -> &[u8] {
        &self.event_data
    }

    pub fn type_tag(&self) -> &TypeTag {
        &self.type_tag
    }

    pub fn size(&self) -> usize {
        self.key.size() + 8 /* u64 */ + bcs::serialized_size(&self.type_tag).unwrap() + self.event_data.len()
    }
}

impl std::fmt::Debug for ContractEventV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ContractEvent {{ key: {:?}, index: {:?}, type: {:?}, event_data: {:?} }}",
            self.key,
            self.sequence_number,
            self.type_tag,
            hex::encode(&self.event_data)
        )
    }
}

/// Entry produced via a call to the `emit` builtin.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, CryptoHasher)]
pub struct ContractEventV2 {
    /// The type of the data
    type_tag: TypeTag,
    /// The data payload of the event
    #[serde(with = "serde_bytes")]
    event_data: Vec<u8>,
}

impl ContractEventV2 {
    pub fn new(type_tag: TypeTag, event_data: Vec<u8>) -> Self {
        Self {
            type_tag,
            event_data,
        }
    }

    pub fn size(&self) -> usize {
        bcs::serialized_size(&self.type_tag).unwrap() + self.event_data.len()
    }

    pub fn type_tag(&self) -> &TypeTag {
        &self.type_tag
    }

    pub fn event_data(&self) -> &[u8] {
        &self.event_data
    }
}

impl std::fmt::Debug for ContractEventV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ModuleEvent {{ type: {:?}, event_data: {:?} }}",
            self.type_tag,
            hex::encode(&self.event_data)
        )
    }
}

impl TryFrom<&ContractEvent> for NewBlockEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V1(event) => {
                if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    bail!("Expected NewBlockEvent")
                }
                Self::try_from_bytes(&event.event_data)
            },
            ContractEvent::V2(_) => bail!("This is a module event"),
        }
    }
}

impl TryFrom<&ContractEvent> for DKGStartEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V1(_) => {
                bail!("conversion to dkg start event failed with wrong contract event version");
            },
            ContractEvent::V2(event) => {
                if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    bail!("conversion to dkg start event failed with wrong type tag")
                }
                bcs::from_bytes(&event.event_data).map_err(Into::into)
            },
        }
    }
}

impl TryFrom<&ContractEvent> for NewEpochEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        if event.is_new_epoch_event() {
            Self::try_from_bytes(event.event_data())
        } else {
            bail!("Expected NewEpochEvent")
        }
    }
}

impl TryFrom<&ContractEvent> for WithdrawEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V1(event) => {
                if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    bail!("Expected Sent Payment")
                }
                Self::try_from_bytes(&event.event_data)
            },
            ContractEvent::V2(_) => bail!("This is a module event"),
        }
    }
}

impl TryFrom<&ContractEvent> for DepositEvent {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V1(event) => {
                if event.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    bail!("Expected Received Payment")
                }
                Self::try_from_bytes(&event.event_data)
            },
            ContractEvent::V2(_) => bail!("This is a module event"),
        }
    }
}

impl TryFrom<&ContractEvent> for ObservedJWKsUpdated {
    type Error = Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        match event {
            ContractEvent::V1(_) => {
                bail!("conversion to `ObservedJWKsUpdated` failed with wrong event version")
            },
            ContractEvent::V2(v2) => {
                if v2.type_tag != TypeTag::Struct(Box::new(Self::struct_tag())) {
                    bail!("conversion to `ObservedJWKsUpdated` failed with wrong type tag");
                }
                bcs::from_bytes(&v2.event_data).map_err(Into::into)
            },
        }
    }
}

impl std::fmt::Debug for ContractEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractEvent::V1(event) => event.fmt(f),
            ContractEvent::V2(event) => event.fmt(f),
        }
    }
}

impl std::fmt::Display for ContractEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(payload) = WithdrawEvent::try_from(self) {
            let v1 = self.v1().unwrap();
            write!(
                f,
                "ContractEvent {{ key: {}, index: {:?}, type: {:?}, event_data: {:?} }}",
                v1.key, v1.sequence_number, v1.type_tag, payload,
            )
        } else if let Ok(payload) = DepositEvent::try_from(self) {
            let v1 = self.v1().unwrap();
            write!(
                f,
                "ContractEvent {{ key: {}, index: {:?}, type: {:?}, event_data: {:?} }}",
                v1.key, v1.sequence_number, v1.type_tag, payload,
            )
        } else {
            write!(f, "{:?}", self)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct EventWithVersion {
    pub transaction_version: Version,
    pub event: ContractEvent,
}

impl std::fmt::Display for EventWithVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EventWithVersion {{ \n\ttransaction_version: {}, \n\tevent: {} \n}}",
            self.transaction_version, self.event
        )
    }
}

impl EventWithVersion {
    /// Constructor.
    pub fn new(transaction_version: Version, event: ContractEvent) -> Self {
        Self {
            transaction_version,
            event,
        }
    }
}

impl TryFrom<&GravityEvent> for ContractEvent {
    type Error = Error;

    fn try_from(event: &GravityEvent) -> Result<Self> {
        // Note: NewEpoch events use serde_json serialization while JWK/DKG events use BCS.
        // This is intentional — NewEpoch follows the Gravity execution layer's JSON-based
        // event format, while JWK and DKG use BCS to match the Aptos types layer expectations.
        match event {
            GravityEvent::NewEpoch(epoch, _) => {
                let data = NewEpochEvent { epoch: *epoch };
                Ok(ContractEvent::V2(ContractEventV2::new(
                    TypeTag::Struct(Box::new(NewEpochEvent::struct_tag())),
                    bcs::to_bytes(&data).unwrap(),
                )))
            },
            GravityEvent::ObservedJWKsUpdated(epoch, jwks) => {
                let data = ObservedJWKsUpdated {
                    epoch: *epoch,
                    jwks: AllProvidersJWKs {
                        entries: jwks
                            .iter()
                            .map(|jwk| ProviderJWKs {
                                issuer: jwk.issuer.clone(),
                                version: jwk.version,
                                jwks: jwk
                                    .jwks
                                    .iter()
                                    .map(|jwk| {
                                        match jwk.type_name.as_str() {
                                            // TODO(Gravity_byteyue): Support RSA later
                                            RSA_JWK::MOVE_TYPE_NAME => JWKMoveStruct {
                                                variant: Any {
                                                    type_name: RSA_JWK::MOVE_TYPE_NAME.to_string(),
                                                    data: jwk.data.clone(),
                                                },
                                            },
                                            UnsupportedJWK::MOVE_TYPE_NAME => {
                                                let unsupported_jwk = UnsupportedJWK {
                                                    id: UnsupportedJWK::MOVE_TYPE_NAME
                                                        .as_bytes()
                                                        .to_vec(),
                                                    payload: jwk.data.clone(),
                                                };
                                                JWKMoveStruct {
                                                    variant: unsupported_jwk.as_move_any(),
                                                }
                                            },
                                            _ => {
                                                return Err(anyhow::anyhow!(
                                                    "unknown jwk type: {}",
                                                    jwk.type_name
                                                ))
                                            },
                                        }
                                    })
                                    .collect(),
                            })
                            .collect(),
                    },
                };
                Ok(ContractEvent::V2(ContractEventV2::new(
                    TypeTag::Struct(Box::new(ObservedJWKsUpdated::struct_tag())),
                    bcs::to_bytes(&data).unwrap(),
                )))
            },
            GravityEvent::DKG(dkg) => {
                let data = DKGStartEvent {
                    session_metadata: crate::dkg::DKGSessionMetadata::from_api_types(
                        dkg.session_metadata.clone(),
                    )?,
                    start_time_us: dkg.start_time_us,
                };
                Ok(ContractEvent::V2(ContractEventV2::new(
                    TypeTag::Struct(Box::new(crate::dkg::DKGStartEvent::struct_tag())),
                    bcs::to_bytes(&data).unwrap(),
                )))
            },
        }
    }
}

impl From<GravityEvent> for ContractEvent {
    fn from(event: GravityEvent) -> Self {
        ContractEvent::try_from(&event).unwrap()
    }
}
