// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod client;
pub mod types;

#[cfg(test)]
mod test {
    use crate::{
        v2::{
            client::{AptosClient, AptosClientBuilder},
            types::Page,
        },
        AccountAddress,
    };
    use aptos_types::account_config::aptos_root_address;
    use std::str::FromStr;

    // TODO: Make these tests against a running server
    fn setup_client() -> AptosClient {
        AptosClientBuilder::new("http://localhost:8080".parse().unwrap())
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_ledger_info() {
        let client = setup_client();
        let response = client.get_ledger_info().await.unwrap();
        let ledger_info = response.ledger_info();
        assert_eq!(ledger_info, response.inner());
    }

    #[tokio::test]
    async fn health_check() {
        let client = setup_client();
        client.health_check(5).await.unwrap();
    }

    #[tokio::test]
    async fn test_account() {
        let client = setup_client();
        client.get_account(AccountAddress::ONE).await.unwrap();
    }

    #[tokio::test]
    async fn test_account_resources() {
        let client = setup_client();
        let one_response = client
            .get_account_resources(AccountAddress::ONE, Some(1))
            .await
            .unwrap();
        let zero_response = client
            .get_account_resources(AccountAddress::ONE, Some(0))
            .await
            .unwrap();
        let now_response = client
            .get_account_resources(AccountAddress::ONE, None)
            .await
            .unwrap();

        assert_eq!(zero_response.inner(), one_response.inner());
        assert_eq!(zero_response.inner(), now_response.inner());
    }

    #[tokio::test]
    async fn test_account_resource() {
        let client = setup_client();
        let one_response = client
            .get_account_resource(
                AccountAddress::ONE,
                "0x1::Coin::CoinInfo<0x1::TestCoin::TestCoin>",
                Some(1),
            )
            .await
            .unwrap();
        let zero_response = client
            .get_account_resource(
                AccountAddress::ONE,
                "0x1::Coin::CoinInfo<0x1::TestCoin::TestCoin>",
                Some(0),
            )
            .await
            .unwrap();
        let now_response = client
            .get_account_resource(
                AccountAddress::ONE,
                "0x1::Coin::CoinInfo<0x1::TestCoin::TestCoin>",
                None,
            )
            .await
            .unwrap();

        assert_eq!(zero_response.inner(), one_response.inner());
        assert_eq!(zero_response.inner(), now_response.inner());
    }

    #[tokio::test]
    async fn test_account_modules() {
        let client = setup_client();
        let one_response = client
            .get_account_modules(AccountAddress::ONE, Some(1))
            .await
            .unwrap();
        let zero_response = client
            .get_account_modules(AccountAddress::ONE, Some(0))
            .await
            .unwrap();
        let now_response = client
            .get_account_modules(AccountAddress::ONE, None)
            .await
            .unwrap();

        assert_eq!(zero_response.inner(), one_response.inner());
        assert_eq!(zero_response.inner(), now_response.inner());
    }

    #[tokio::test]
    async fn test_account_transactions() {
        let client = setup_client();
        let response = client
            .get_account_transactions(aptos_root_address(), None)
            .await
            .unwrap();

        assert!(!response.inner().is_empty());
        let txn = response.inner().first().unwrap();
        let response = client
            .get_account_transactions(aptos_root_address(), Some(Page::new(Some(0), Some(1))))
            .await
            .unwrap();

        assert_eq!(response.inner().first().unwrap(), txn);
    }

    #[tokio::test]
    async fn test_account_transaction() {
        let client = setup_client();
        let response = client
            .get_account_transactions(aptos_root_address(), None)
            .await
            .unwrap();
        let txn = response.inner().first().unwrap();
        let txn_info = txn.transaction_info().unwrap();
        let by_version = client
            .get_transaction_by_version(txn_info.version.0)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(&by_version, txn);

        let by_hash = client
            .get_transaction_by_hash(txn_info.hash.into())
            .await
            .unwrap()
            .into_inner();
        assert_eq!(&by_hash, txn);
    }

    #[tokio::test]
    async fn get_account_balance() {
        let client = setup_client();
        let account = AccountAddress::from_str(
            "cb48471868293a5feb8fac3871775b62a671b7c405b43e7755f368901df9ec8c",
        )
        .unwrap();
        let balance = client.get_account_balance(account, None).await.unwrap();
        assert!(balance.coin.value.0 > 0);
    }
}
