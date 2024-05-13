module resource_account::resource_signer_holder {
	use aptos_std::signer;
	use aptos_std::code;
    use aptos_framework::account;
    use aptos_framework::resource_account;
	use resource_account::bonding_curve_launchpad;
	use resource_account::liquidity_pair;
	friend bonding_curve_launchpad;
	friend liquidity_pair;

	const ENON_OWNER_ACCOUNT: u64 = 8030000000;


	struct Config has key {
	    owner: address,
	    signer_cap: account::SignerCapability
	}

	const OWNER: address = @owner_addr;
	const RESOURCE_ACCOUNT: address = @resource_account;

	fun init_module(resource_account: &signer)  {
	    // Must create this struct on constructor level, as we don't get the signer back when we create a resource account.
	    // The signer_cap was created from creating the resource account prior to creating this contract.
	    let signer_cap = resource_account::retrieve_resource_account_cap(resource_account, OWNER);
	    move_to(resource_account, Config {
	        owner: OWNER,
	        signer_cap
	    });
	}

	public entry fun upgrade(
		owner: &signer,
		metadata_serialized: vector<u8>,
		code: vector<vector<u8>>
	) acquires Config {
		let config = borrow_global<Config>(RESOURCE_ACCOUNT);
		assert!(config.owner == signer::address_of(owner), ENON_OWNER_ACCOUNT);

		let signer = account::create_signer_with_capability(&config.signer_cap);
		code::publish_package_txn(&signer, metadata_serialized, code);
	}

	#[view]
    public(friend) fun get_signer(): signer acquires Config {
        let signer_cap = &borrow_global<Config>(RESOURCE_ACCOUNT).signer_cap;
        account::create_signer_with_capability(signer_cap)
    }
	#[view]
	public(friend) fun get_owner(): address acquires Config {
		let owner = borrow_global<Config>(RESOURCE_ACCOUNT).owner;
        owner
	}

	//---------------------------Tests---------------------------
    #[test_only]
    public fun initialize_for_test(deployer: &signer){
	    // let signer_cap = resource_account::retrieve_resource_account_cap(deployer, OWNER);
	    move_to(deployer, Config {
	        owner: OWNER,
	        signer_cap: account::create_test_signer_cap(signer::address_of(deployer)),
	    });
    }

}
