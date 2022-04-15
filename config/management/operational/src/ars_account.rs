// use aptos_types::{
//     account_address::AccountAddress,
//     transaction::{authenticator::AuthenticationKey, Transaction},
// };

use aptos_crypto::{ValidCryptoMaterial, x25519};

use crate::ars_account_client::{
    ArsAccount
};
use crate::keys::{ EncodingType, load_key };
use std::{
    path::{PathBuf},
};
use aptos_management::{error::Error};
use structopt::StructOpt;
use serde::Serialize;
// pub const TESTNET_URL: &str = "https://fullnode.devnet.aptoslabs.com";
// pub const FAUCET_URL: &str = "https://faucet.devnet.aptoslabs.com";

#[derive(Debug, StructOpt)]
pub struct ArsAccountAddress {
    // /// JSON-RPC Endpoint (e.g. http://localhost:8080)
    // #[structopt(long, required_unless = "config")]
    // json_server: Option<String>,
    #[structopt(long)]
    key_file: PathBuf,
    #[structopt(long)]
    encoding: EncodingType,
}

#[derive(Debug, Serialize)]
pub struct ArsAccountAddressResource{
    pub ars_account_address: String,
    pub public_key: String,

}

impl ArsAccountAddress{
    pub async fn execute(self) ->  Result<ArsAccountAddressResource, Error> {
        let private_key = load_key::<x25519::PrivateKey>(self.key_file, self.encoding)?;
        let new_account = ArsAccount::new(Some(private_key.to_bytes()));
        println!("\n=== Addresses ===");
        println!("new_account: 0x{}", new_account.address());
        Ok(ArsAccountAddressResource{
            ars_account_address: new_account.address(),
            public_key: new_account.pub_key()
        })
    }
}


