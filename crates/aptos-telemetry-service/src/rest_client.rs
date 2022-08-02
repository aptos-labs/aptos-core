use anyhow::{anyhow, Result};
use aptos_rest_client::{Client, Response, state::State};
use aptos_types::{
    account_address::AccountAddress, account_config::CORE_CODE_ADDRESS,
};
use serde::de::DeserializeOwned;

use crate::types::validator_set::{ValidatorSet,ValidatorInfo};

#[derive(Clone)]
pub struct RestClient {
    client: Client,
}

impl RestClient {
    pub fn new(api_url: String) -> Self {
        Self {
            client: Client::new(url::Url::parse(&api_url).unwrap()),
        }
    }

    async fn get_resource<T: DeserializeOwned>(
        &self,
        address: AccountAddress,
        resource_type: &str,
    ) -> Result<Response<T>> {
        return self.client.get_resource(address, resource_type).await;
    }

    pub async fn validator_set(&self) -> Result<(Vec<ValidatorInfo>, State)> {
        let (validator_set, state): (ValidatorSet, State) = self
            .get_resource(CORE_CODE_ADDRESS, "0x1::stake::ValidatorSet")
            .await?
            .into_parts();

        let mut validator_infos = vec![];
        for validator_info in validator_set.payload() {
            validator_infos.push(validator_info.clone());
        }

        if validator_infos.is_empty() {
            return Err(anyhow!("No validator sets were found!"));
        }
        Ok((validator_infos, state))
    }
}
