// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod client;
pub mod types;

#[cfg(test)]
mod test {
    use super::*;
    use crate::v2::client::{AptosClient, AptosClientBuilder};

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
}
