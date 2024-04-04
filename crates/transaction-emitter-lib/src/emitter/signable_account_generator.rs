use aptos_sdk::types::{AccountKey, LocalAccount, SignableAccount};
use aptos_transaction_generator_lib::{AccountType, ReliableTransactionSubmitter};
use async_trait::async_trait;
use futures::future::try_join_all;
use rand::rngs::StdRng;

#[async_trait]
pub trait SignableAccountGenerator {
    async fn gen_signable_accounts(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        rng: &mut StdRng,
    ) -> anyhow::Result<Vec<Box<dyn SignableAccount>>>;
}

pub fn create_account_generator(account_type: AccountType) -> Box<dyn SignableAccountGenerator> {
    match account_type {
        AccountType::Local => Box::new(LocalAccountGenerator),
        _ => {
            unimplemented!("Account type {:?} is not supported", account_type)
        },
    }
}

pub struct LocalAccountGenerator;

#[async_trait]
impl SignableAccountGenerator for LocalAccountGenerator {
    async fn gen_signable_accounts(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        rng: &mut StdRng,
    ) -> anyhow::Result<Vec<Box<dyn SignableAccount>>> {
        let mut account_keys = vec![];
        let mut addresses = vec![];
        let mut i = 0;
        while i < num_accounts {
            let account_key = AccountKey::generate(rng);
            addresses.push(account_key.authentication_key().account_address());
            account_keys.push(account_key);
            i += 1;
        }
        let result_futures = addresses
            .iter()
            .map(|address| txn_executor.query_sequence_number(*address))
            .collect::<Vec<_>>();
        let seq_nums: Vec<_> = try_join_all(result_futures).await?.into_iter().collect();

        let accounts = account_keys
            .into_iter()
            .zip(seq_nums)
            .map(|(account_key, sequence_number)| {
                Box::new(LocalAccount::new(
                    account_key.authentication_key().account_address(),
                    account_key,
                    sequence_number,
                )) as Box<dyn SignableAccount>
            })
            .collect();
        Ok(accounts)
    }
}
