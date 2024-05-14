//**
//* This resource account is used for storing a signer var.
//* The signer var is re-used during FA creation, to keep the FA on the smart contract platform (rather than on the creator's account).
module resource_account::resource_signer_holder {
	use aptos_std::signer;
	use aptos_std::code;
    use aptos_framework::account;
    use aptos_framework::resource_account;
	use resource_account::bonding_curve_launchpad;
	use resource_account::liquidity_pair;
	friend bonding_curve_launchpad;
	friend liquidity_pair;

	struct Config has key {
	    owner: address,
	    signer_cap: account::SignerCapability
	}

	const OWNER: address = @owner_addr;
	const RESOURCE_ACCOUNT: address = @resource_account;

	const ENON_OWNER_ACCOUNT: u64 = 8030000000;

	fun init_module(resource_account: &signer)  {
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

    //---------------------------Views---------------------------
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
	    move_to(deployer, Config {
	        owner: OWNER,
	        signer_cap: account::create_test_signer_cap(signer::address_of(deployer)),
	    });
    }
}
