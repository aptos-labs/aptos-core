module aptos_framework::fee_destribution {
    use std::error;
    use std::option::{Self, Option};

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, AggregatorCoin};
    use aptos_framework::system_addresses;

    /// When struct holding distribution ifnormation already exists.
    const EDISTRIBUTION_INFO_EXISTS: u64 = 1;

    /// Resource which holds the collected transaction fees and also the proposer
    /// of the block.
    struct DistributionInfo has key {
        balance: AggregatorCoin<AptosCoin>,
        proposer: Option<address>,
    }

    const MAX_U64: u128 = 18446744073709551615;

    /// Initializes the resource holding information for gas fees distribution.
    /// Should be called by on-chain governance.
    public fun initialize_distribution_info(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<DistributionInfo>(@aptos_framework),
            error::already_exists(EDISTRIBUTION_INFO_EXISTS)
        );

        let zero = coin::initialize_aggregator_coin(aptos_framework, MAX_U64);
        let info = DistributionInfo {
            balance: zero,
            proposer: option::none(),
        };
        move_to(aptos_framework, info);
    }
}
