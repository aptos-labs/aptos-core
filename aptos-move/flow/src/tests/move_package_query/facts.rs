// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_facts() {
    let pkg = common::make_package("facts", &[(
        "coin",
        "module 0xCAFE::coin {
    friend 0xCAFE::coin_admin;

    const E_NOT_REGISTERED: u64 = 1;

    struct CoinStore<phantom CoinType> has key {
        balance: u64,
    }

    #[event]
    struct TransferEvent has drop, store {
        amount: u64,
    }

    enum Status has drop, store {
        Active,
        Frozen { reason: u64 },
        Pending(u64, u64),
    }

    public entry fun register<CoinType>(account: &signer) {
        move_to(account, CoinStore<CoinType> { balance: 0 });
    }

    #[view]
    public fun balance_of<CoinType>(addr: address): u64 acquires CoinStore {
        borrow_global<CoinStore<CoinType>>(addr).balance
    }
}

module 0xCAFE::coin_admin {
    public(friend) fun freeze_account(_addr: address) {}
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
