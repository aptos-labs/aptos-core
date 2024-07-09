module aptos_framework::next_validator_set {
    use std::option;
    use std::option::Option;
    use aptos_framework::validator_consensus_info::ValidatorConsensusInfo;
    friend aptos_framework::stake;
    friend aptos_framework::reconfiguration;
    friend aptos_framework::mpc;

    struct NextValidatorSet has key {
        next_validator_set: Option<vector<ValidatorConsensusInfo>>,
    }

    public fun initialize(framework: &signer) {
        if (!exists<NextValidatorSet>(@aptos_framework)) {
            move_to(framework, NextValidatorSet { next_validator_set: option::none() } )
        }
    }

    public(friend) fun save(infos: vector<ValidatorConsensusInfo>) acquires NextValidatorSet {
        borrow_global_mut<NextValidatorSet>(@aptos_framework).next_validator_set = option::some(infos);
    }

    public(friend) fun clear() acquires NextValidatorSet {
        borrow_global_mut<NextValidatorSet>(@aptos_framework).next_validator_set = option::none();
    }

    public(friend) fun load(): vector<ValidatorConsensusInfo> acquires NextValidatorSet {
        let maybe_set = borrow_global<NextValidatorSet>(@aptos_framework).next_validator_set;
        option::extract(&mut maybe_set)
    }
}
