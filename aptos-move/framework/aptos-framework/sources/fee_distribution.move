module aptos_framework::fee_destribution {
    use std::error;
    use std::option::{Self, Option};

    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, AggregatorCoin};
    use aptos_framework::system_addresses;
    use aptos_framework::transaction_fee;

    friend aptos_framework::transaction_validation;

    /// When struct holding distribution ifnormation already exists.
    const EDISTRIBUTION_INFO_EXISTS: u64 = 1;

    /// Resource which holds the collected transaction fees and their receiver.
    struct DistributionInfo has key {
        balance: AggregatorCoin<AptosCoin>,
        receiver: Option<address>,
    }

    /// Initializes the resource holding information for gas fees distribution.
    /// Should be called by on-chain governance.
    public fun initialize_distribution_info(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<DistributionInfo>(@aptos_framework),
            error::already_exists(EDISTRIBUTION_INFO_EXISTS)
        );

        let zero = coin::initialize_aggregator_coin(aptos_framework);
        let info = DistributionInfo {
            balance: zero,
            receiver: option::none(),
        };
        move_to(aptos_framework, info);
    }

    /// Called by transaction epilogue to collect the gas fees from the specified account.
    public(friend) fun collect_fee(account: address, fee: u64) acquires DistributionInfo {
        let distribution_info = borrow_global_mut<DistributionInfo>(@aptos_framework);
        let dst_coin = &mut distribution_info.balance;
        coin::collect_from(account, fee, dst_coin);
    }

    /// Distributes collected transaction fees to the receiver. Should be called
    /// at the beginning of each block.
    public fun distribute(vm: &signer) acquires DistributionInfo {
        // Can only be called by the VM.
        system_addresses::assert_vm(vm);
        let distribution_info = borrow_global_mut<DistributionInfo>(@aptos_framework);

        // First, do nothing if there are no collected fees.
        if (coin::is_zero(&distribution_info.balance)) {
            return
        };

        let coin = coin::drain(&mut distribution_info.balance);
        if (option::is_some(&distribution_info.receiver)) {
            let receiver_addr = *option::borrow(&distribution_info.receiver);

            // There is a receiver, but it might not have account registered for storing
            // coins, so check for that.
            let receiver_has_account = coin::is_account_registered<AptosCoin>(receiver_addr);
            if (receiver_has_account) {
                // If all checks passed, deposit coins to the receiver's account.
                coin::deposit(receiver_addr, coin);
                return
            };
        };

        // Otherwise, burn the collected coins.
        transaction_fee::burn_collected_fee(coin);
    }
}
