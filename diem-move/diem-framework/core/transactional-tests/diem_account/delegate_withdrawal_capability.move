//# init --parent-vasps Alice Bob Carrol

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Give Bob some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Bob 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Give Carrol some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Carrol 10000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::SillyColdWallet {
    use DiemFramework::XUS::XUS;
    use DiemFramework::DiemAccount;
    use DiemFramework::Diem;
    use Std::Signer;

    struct T has key {
        cap: DiemAccount::WithdrawCapability,
        owner: address,
    }

    public(script) fun publish(account: signer, owner: address) {
        let cap = DiemAccount::extract_withdraw_capability(&account);
        move_to(&account, T { cap, owner });
    }

    public fun withdraw(account: &signer, wallet_address: address, _amount: u64): Diem::Diem<XUS> acquires T {
        let wallet_ref = borrow_global_mut<T>(wallet_address);
        let sender = Signer::address_of(account);
        assert!(wallet_ref.owner == sender, 77);
        // TODO: the withdraw_from API is no longer exposed in DiemAccount
        Diem::zero()
    }
}

//# run --signers Alice --args @Bob -- 0xA550C18::SillyColdWallet::publish

// Check that Alice can no longer withdraw from her account.
//
//# run --type-args 0x1::XUS::XUS --signers Alice --args @Alice 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Check that Bob can still withdraw from his normal account.
//
//# run --type-args 0x1::XUS::XUS --signers Bob --args @Bob 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

// Check that other users can still pay into Alice's account in the normal way.
//
//# run --type-args 0x1::XUS::XUS --signers Carrol --args @Alice 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata
