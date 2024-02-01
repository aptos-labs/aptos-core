module 0x1::staking_proxy {
    public entry fun set_operator(arg0: &signer, arg1: address, arg2: address) {
        set_vesting_contract_operator(arg0, arg1, arg2);
        set_staking_contract_operator(arg0, arg1, arg2);
        set_stake_pool_operator(arg0, arg2);
    }
    
    public entry fun set_stake_pool_operator(arg0: &signer, arg1: address) {
        if (0x1::stake::stake_pool_exists(0x1::signer::address_of(arg0))) {
            0x1::stake::set_operator(arg0, arg1);
        };
    }
    
    public entry fun set_stake_pool_voter(arg0: &signer, arg1: address) {
        if (0x1::stake::stake_pool_exists(0x1::signer::address_of(arg0))) {
            0x1::stake::set_delegated_voter(arg0, arg1);
        };
    }
    
    public entry fun set_staking_contract_operator(arg0: &signer, arg1: address, arg2: address) {
        let v0 = 0x1::signer::address_of(arg0);
        if (0x1::staking_contract::staking_contract_exists(v0, arg1)) {
            let v1 = 0x1::staking_contract::commission_percentage(v0, arg1);
            0x1::staking_contract::switch_operator(arg0, arg1, arg2, v1);
        };
    }
    
    public entry fun set_staking_contract_voter(arg0: &signer, arg1: address, arg2: address) {
        if (0x1::staking_contract::staking_contract_exists(0x1::signer::address_of(arg0), arg1)) {
            0x1::staking_contract::update_voter(arg0, arg1, arg2);
        };
    }
    
    public entry fun set_vesting_contract_operator(arg0: &signer, arg1: address, arg2: address) {
        let v0 = 0x1::vesting::vesting_contracts(0x1::signer::address_of(arg0));
        let v1 = &v0;
        let v2 = 0;
        while (v2 < 0x1::vector::length<address>(v1)) {
            let v3 = *0x1::vector::borrow<address>(v1, v2);
            if (0x1::vesting::operator(v3) == arg1) {
                0x1::vesting::update_operator(arg0, v3, arg2, 0x1::vesting::operator_commission_percentage(v3));
            };
            v2 = v2 + 1;
        };
    }
    
    public entry fun set_vesting_contract_voter(arg0: &signer, arg1: address, arg2: address) {
        let v0 = 0x1::vesting::vesting_contracts(0x1::signer::address_of(arg0));
        let v1 = &v0;
        let v2 = 0;
        while (v2 < 0x1::vector::length<address>(v1)) {
            let v3 = *0x1::vector::borrow<address>(v1, v2);
            if (0x1::vesting::operator(v3) == arg1) {
                0x1::vesting::update_voter(arg0, v3, arg2);
            };
            v2 = v2 + 1;
        };
    }
    
    public entry fun set_voter(arg0: &signer, arg1: address, arg2: address) {
        set_vesting_contract_voter(arg0, arg1, arg2);
        set_staking_contract_voter(arg0, arg1, arg2);
        set_stake_pool_voter(arg0, arg2);
    }
    
    // decompiled from Move bytecode v6
}
