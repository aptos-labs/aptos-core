use std::collections::HashMap;
use crate::common::types::{CliCommand, CliResult};
use clap::Subcommand;
use std::path::PathBuf;
use aptos_config::config::Token;
use aptos_types::account_address::AccountAddress;

pub mod create;

/// Tool for interacting with NFTs
///
/// This tool is used to
/// 1. create batch NFTs from a folder of media files and metadata
/// 2. upload NFT

#[derive(Debug, Subcommand)]
pub enum NFTTool {
    Create(create::CreateNFT),
}

#[derive(Debug, Clone)]
pub struct CreatorShares {
    // the key is the address of the creator and value is weight
    creator_shares: HashMap<String, AccountAddress>,
}

/// the input of one NFT to be created
#[derive(Debug, Clone)]
pub struct NFTEntry {
    name: String,
    description: String,
    amount: u64, // the number of tokens to be created
    maximum: u64,
    token_file_path: PathBuf, // the path to the media file of the token
    creator_shares: CreatorShares,
    royalty_points_denominator: u64,
    royalty_points_numerator: u64,
    token_mutate_setting: Vec<bool>,
    property_keys: Vec<String>,
    property_values: Vec<Vec<u8>>,
    property_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CollectionEntry {
    name: String,
    description: String,
    collection_file_path: PathBuf,
    maximum: u64,
    mutate_setting: Vec<bool>,
    tokens: Vec<NFTEntry>,
}

#[derive(Debug, Clone)]
pub struct CollectionsEntry {
    collections: Vec<CollectionEntry>,
}