// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource account seed parsing and encoding for the `aptos move create-resource-account-and-publish-package` command.

use aptos_cli_common::{CliError, CliTypedResult};
use aptos_rest_client::aptos_api_types::HexEncodedBytes;
use clap::Parser;
use std::{fmt::Formatter, str::FromStr};

/// Encoding for the Resource account seed
#[derive(Debug, Default, Clone, Copy)]
pub enum SeedEncoding {
    #[default]
    Bcs,
    Hex,
    Utf8,
}

const BCS: &str = "bcs";
const UTF_8: &str = "utf8";
const HEX: &str = "hex";

impl std::fmt::Display for SeedEncoding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SeedEncoding::Bcs => BCS,
            SeedEncoding::Hex => HEX,
            SeedEncoding::Utf8 => UTF_8,
        })
    }
}

impl FromStr for SeedEncoding {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            BCS => Ok(Self::Bcs),
            HEX => Ok(Self::Hex),
            UTF_8 | "utf-8" | "utf_8" => Ok(Self::Utf8),
            _ => Err(CliError::UnableToParse(
                "seed-encoding",
                "For --seed-encoding please provide one of ['bcs','hex', 'utf8']".to_string(),
            )),
        }
    }
}

/// A generic interface for allowing for different types of seed phrase inputs
///
/// The easiest to use is `string_seed` as it will match directly with the b"string" notation in Move.
#[derive(Debug, Parser)]
pub struct ResourceAccountSeed {
    /// Resource account seed
    ///
    /// Seed used in generation of the AccountId of the resource account
    /// The seed will be converted to bytes using the encoding from `--seed-encoding`, defaults to `BCS`
    #[clap(long)]
    pub seed: String,

    /// Resource account seed encoding
    ///
    /// The encoding can be one of `Bcs`, `Utf8`, and `Hex`.
    ///
    /// - Bcs is the legacy functionality of the CLI, it will BCS encode the string, but can be confusing for users e.g. `"ab" -> vector<u8>[0x2, 0x61, 0x62]`
    /// - Utf8 will encode the string as raw UTF-8 bytes, similar to in Move `b"string"` e.g. `"ab" -> vector<u8>[0x61, 0x62]`
    /// - Hex will encode the string as raw hex encoded bytes e.g. `"0x6162" -> vector<u8>[0x61, 0x62]`
    #[clap(long, default_value_t = SeedEncoding::Bcs)]
    pub seed_encoding: SeedEncoding,
}

impl ResourceAccountSeed {
    pub fn seed(self) -> CliTypedResult<Vec<u8>> {
        match self.seed_encoding {
            SeedEncoding::Bcs => Ok(bcs::to_bytes(self.seed.as_str())?),
            SeedEncoding::Utf8 => Ok(self.seed.as_bytes().to_vec()),
            SeedEncoding::Hex => HexEncodedBytes::from_str(self.seed.as_str())
                .map(|inner| inner.0)
                .map_err(|err| CliError::UnableToParse("seed", err.to_string())),
        }
    }
}
