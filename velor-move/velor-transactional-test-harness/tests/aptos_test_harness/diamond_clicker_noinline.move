//# init --addresses Alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys Alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key Alice
module Alice::game {

    // #[test_only]
    use velor_std::debug;
    // #[test_only]
    use std::signer;
    // #[test_only]
    use std::vector;

    struct InnerStruct has key, store, copy {
        amount: u64
    }

    struct OuterStruct has key {
        any_field: vector<InnerStruct>
    }

    // #[test(account=Alice)]
    public entry fun test_upgrade(account: &signer) acquires OuterStruct {
        let account_address = signer::address_of(account);
        // let upgrade_amount = 0;
        move_to(account, OuterStruct {any_field: vector::empty()});
        let anystruct = borrow_global_mut<OuterStruct>(account_address);
	{
            let v: &mut vector<InnerStruct> = &mut anystruct.any_field;
	    let i: u64 = 0;
	    let len: u64 = vector::length(v);
	    while (i < len) {
		{
		    let field = vector::borrow_mut(v, i);
		    {
			debug::print(field); // INTERNAL TEST ERROR: INTERNAL VM INVARIANT VIOLATION
			// debug::print(b"foo"); // INTERNAL TEST ERROR: INTERNAL VM INVARIANT VIOLATION
			// let field: &mut InnerStruct = field;
			// field.amount = field.amount + upgrade_amount;
		    }
		};
		i = i + 1
	    }
	}
    }
}

//# run Alice::game::test_upgrade --signers Alice --private-key Alice
