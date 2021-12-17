//# init --parent-vasps Bob
//#      --addresses Alice=0xeaad84d8728031f145581a70310ca79a
//#      --private-keys Alice=9fcb97240842bfd1089b169040305ae1a0c8b786c2db9cbece85dabe562f3dea

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0
//#            @Alice
//#            x"10af6196879ea258e09f83aef42e0c85"
//#            b"alice"
//#            false
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

//# publish
module DiemRoot::Test {
    public(script) fun nop() {}
}

//# publish
module DiemRoot::SharedKeyRotation {
    use DiemFramework::DiemAccount;
    use Std::Signer;

    struct T has key {
        // cap.address can rotate the auth key for cap.address
        cap: DiemAccount::KeyRotationCapability,
        // master_key_address can also rotate the auth key for cap.address
        master_key_address: address,
    }

    // Publish a SharedRotation resource for the account `cap.address` with master key
    // `master_key_address` under the sender's account
    public(script) fun publish(account: signer, master_key_address: address) {
        let cap = DiemAccount::extract_key_rotation_capability(&account);
        move_to(&account, T { cap, master_key_address });
    }

    // Rotate the auth key for the account at wallet_address/SharedKeyRotation.SharedKeyRotation/cap/address to
    // new_key
    public(script) fun rotate(account: signer, wallet_address: address, new_key: vector<u8>) acquires T {
        let wallet_ref = borrow_global_mut<T>(wallet_address);
        let sender = Signer::address_of(&account);
        let cap_addr = *DiemAccount::key_rotation_capability_address(&wallet_ref.cap);
        assert!((wallet_ref.master_key_address == sender) || (cap_addr == sender), 77);
        DiemAccount::rotate_authentication_key(&wallet_ref.cap, new_key);
    }
}

// Create a SharedKeyRotation for Alice's account with Bob's account key as the master key
//
//# run --signers Alice --args @Bob -- 0xA550C18::SharedKeyRotation::publish


// Alice can rotate her key. Here, she rotates it to its original value
//
//# run --signers Alice --args @Alice x"10af6196879ea258e09f83aef42e0c85eaad84d8728031f145581a70310ca79a"
//#     -- 0xA550C18::SharedKeyRotation::rotate

// Bob can too. Here, he zeroes it out to stop Alice from sending any transactions
//
//# run --signers Bob --args @Alice x"0000000000000000000000000000000000000000000000000000000000000000"
//#     -- 0xA550C18::SharedKeyRotation::rotate

// Alice should no longer be able to send a tx from her account
//
//# run --signers Alice -- 0xA550C18::Test::nop

// Bob now rotates the key back to its old value
//
//# run --signers Bob --args @Alice x"10af6196879ea258e09f83aef42e0c85eaad84d8728031f145581a70310ca79a"
//#     -- 0xA550C18::SharedKeyRotation::rotate

// And then Alice should be able to send a tx once again
//
//# run --signers Alice -- 0xA550C18::Test::nop
