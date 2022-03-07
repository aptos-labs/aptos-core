//# init --parent-vasps Alice Bob Carrol

// Give Alice some money...
//# run --type-args 0x1::XUS::XUS --signers DesignatedDealer --args @Alice 1000 x"" x""
//#     -- 0x1::PaymentScripts::peer_to_peer_with_metadata

//# publish
module DiemRoot::AlicePays {
    use DiemFramework::XUS::XUS;
    use DiemFramework::DiemAccount;

    struct T has key {
        cap: DiemAccount::WithdrawCapability,
    }

    public(script) fun create(sender: signer) {
        move_to(&sender, T {
            cap: DiemAccount::extract_withdraw_capability(&sender),
        })
    }

    public(script) fun pay(payee: address, amount: u64) acquires T {
        let t = borrow_global<T>(@Alice);
        DiemAccount::pay_from<XUS>(
            &t.cap,
            payee,
            amount,
            x"0A11CE",
            x""
        )
    }
}

//# run --signers Alice -- 0xA550C18::AlicePays::create

//# run --signers Bob --args @Carrol 10 -- 0xA550C18::AlicePays::pay
