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

    struct Receipt has drop, store {
        amount: u64,
    }

    #[event]
    struct TransferEvent has drop, store {
        amount: u64,
    }

    #[resource_group(scope = global)]
    struct ObjectGroup {}

    #[resource_group_member(group = 0xCAFE::coin::ObjectGroup)]
    struct Wrapped has key {
        value: u64,
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

    #[view]
    public fun is_registered<CoinType>(addr: address): bool {
        exists<CoinStore<CoinType>>(addr)
    }

    public fun deposit<CoinType>(addr: address, amount: u64) acquires CoinStore {
        if (exists<CoinStore<CoinType>>(addr)) {
            borrow_global_mut<CoinStore<CoinType>>(addr).balance = amount;
        }
    }

    public fun unregister<CoinType>(addr: address): u64 acquires CoinStore {
        let CoinStore<CoinType> { balance } = move_from<CoinStore<CoinType>>(addr);
        balance
    }

    public fun split(amount: u64): (u64, u64) {
        (amount, amount)
    }
}

module 0xCAFE::coin_admin {
    use 0xCAFE::coin::Receipt;

    struct Holder has drop, store {
        r: Receipt,
    }

    public(friend) fun freeze_account(_addr: address) {}

    public fun forward(r: Receipt): Receipt {
        r
    }
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
