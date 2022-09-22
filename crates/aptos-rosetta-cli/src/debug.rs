// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::HexEncodedBytes;
use aptos_types::account_address::AccountAddress;
use aptos_types::network_address::NetworkAddress;
use aptos_types::transaction::{RawTransaction, SignedTransaction};
use clap::{Parser, Subcommand};
use std::str::FromStr;

/// Debug APIs
///
/// Used for debugging Rosetta in general
#[derive(Debug, Subcommand)]
pub enum DebugCommand {
    EncodeBcs(EncodeBcs),
    DecodeBcs(DecodeBcs),
}

impl DebugCommand {
    pub async fn execute(self) -> anyhow::Result<String> {
        match self {
            DebugCommand::EncodeBcs(inner) => inner.execute().await,
            DebugCommand::DecodeBcs(inner) => inner.execute().await,
        }
    }
}

/// Encodes a BCS type
///
/// Example: aptos-rosetta-cli debug encode-bcs --type signed-transaction <HexEncodedBytes>
///
/// Note: This will eventually be added to the Aptos CLI, but for now it is to be used at your
/// own will.
#[derive(Debug, Parser)]
pub struct EncodeBcs {
    /// The type to decode with
    #[clap(short = 't', long, value_enum)]
    r#type: Type,
    /// The string representation that you're trying to encode
    input: String,
}

impl EncodeBcs {
    pub async fn execute(self) -> anyhow::Result<String> {
        self.r#type.encode_bcs(&self.input)
    }
}

/// Parses a BCS type
///
/// Example: aptos-rosetta-cli debug decode-bcs --type signed-transaction <HexEncodedBytes>
///
/// Note: This will eventually be added to the Aptos CLI, but for now it is to be used at your
/// own will.
#[derive(Debug, Parser)]
pub struct DecodeBcs {
    /// The type to decode with
    #[clap(short = 't', long, value_enum)]
    r#type: Type,
    /// The hex encoded bytes to decode from BCS
    input: HexEncodedBytes,
}

impl DecodeBcs {
    pub async fn execute(self) -> anyhow::Result<String> {
        self.r#type.decode_bcs(self.input)
    }
}

/// A typing system for parsing BCS
#[derive(clap::ValueEnum, Debug, Clone)]
pub enum Type {
    UnsignedTransaction,
    SignedTransaction,
    AccountAddress,
    NetworkAddress,
    U8,
    U16,
    U32,
    U64,
    U128,
    VecU8,
    VecVecU8,
    VecNetworkAddress,
}

impl Type {
    pub fn encode_bcs(&self, input: &str) -> anyhow::Result<String> {
        let bytes = match self {
            Type::AccountAddress => bcs::to_bytes(&AccountAddress::from_str(input)?)?,
            Type::NetworkAddress => bcs::to_bytes(&NetworkAddress::from_str(input)?)?,
            Type::U8 => bcs::to_bytes(&u8::from_str(input)?)?,
            Type::U16 => bcs::to_bytes(&u16::from_str(input)?)?,
            Type::U32 => bcs::to_bytes(&u32::from_str(input)?)?,
            Type::U64 => bcs::to_bytes(&u64::from_str(input)?)?,
            Type::U128 => bcs::to_bytes(&u128::from_str(input)?)?,
            Type::VecU8 => bcs::to_bytes(&HexEncodedBytes::from_str(input)?.0)?,
            _ => panic!(
                "Unsupported encoding type {:?}.  No FromStr implementation",
                self
            ),
        };

        Ok(HexEncodedBytes::from(bytes).to_string())
    }

    pub fn decode_bcs(&self, input: HexEncodedBytes) -> anyhow::Result<String> {
        Ok(match self {
            Type::UnsignedTransaction => {
                serde_json::to_string_pretty(&bcs::from_bytes::<RawTransaction>(input.inner())?)?
            }
            Type::SignedTransaction => {
                serde_json::to_string_pretty(&bcs::from_bytes::<SignedTransaction>(input.inner())?)?
            }
            Type::AccountAddress => {
                bcs::from_bytes::<AccountAddress>(input.inner())?.to_hex_literal()
            }
            Type::NetworkAddress => bcs::from_bytes::<NetworkAddress>(input.inner())?.to_string(),
            Type::VecNetworkAddress => serde_json::to_string_pretty(&bcs::from_bytes::<
                Vec<NetworkAddress>,
            >(input.inner())?)?,
            Type::U8 => bcs::from_bytes::<u8>(input.inner())?.to_string(),
            Type::U16 => bcs::from_bytes::<u8>(input.inner())?.to_string(),
            Type::U32 => bcs::from_bytes::<u8>(input.inner())?.to_string(),
            Type::U64 => bcs::from_bytes::<u64>(input.inner())?.to_string(),
            Type::U128 => bcs::from_bytes::<u128>(input.inner())?.to_string(),
            Type::VecU8 => hex::encode(bcs::from_bytes::<Vec<u8>>(input.inner())?),
            Type::VecVecU8 => {
                let vector: Vec<String> = bcs::from_bytes::<Vec<Vec<u8>>>(input.inner())?
                    .into_iter()
                    .map(hex::encode)
                    .collect();
                serde_json::to_string_pretty(&vector)?
            }
        })
    }
}
