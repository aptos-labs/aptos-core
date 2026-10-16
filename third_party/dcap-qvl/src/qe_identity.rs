use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[cfg(feature = "borsh_schema")]
use borsh::BorshSchema;
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};

use crate::tcb_info::TcbStatus;

/// QE Identity structure as returned by Intel's PCCS
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct QeIdentity {
    pub id: String,
    pub version: u8,
    pub issue_date: String,
    pub next_update: String,
    pub tcb_evaluation_data_number: u32,
    #[serde(with = "serde_bytes")]
    pub miscselect: [u8; 4],
    #[serde(rename = "miscselectMask")]
    #[serde(with = "serde_bytes")]
    pub miscselect_mask: [u8; 4],
    #[serde(with = "serde_bytes")]
    pub attributes: [u8; 16],
    #[serde(rename = "attributesMask")]
    #[serde(with = "serde_bytes")]
    pub attributes_mask: [u8; 16],
    #[serde(with = "serde_bytes")]
    pub mrsigner: [u8; 32],
    /// ISV Product ID
    pub isvprodid: u16,
    /// TCB levels for the QE
    pub tcb_levels: Vec<QeTcbLevel>,
}

/// TCB level for QE Identity
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct QeTcbLevel {
    pub tcb: QeTcb,
    pub tcb_date: String,
    pub tcb_status: TcbStatus,
    #[serde(rename = "advisoryIDs", default)]
    pub advisory_ids: Vec<String>,
}

/// TCB component for QE Identity
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct QeTcb {
    pub isvsvn: u16,
}
