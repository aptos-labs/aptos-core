spec aptos_framework::governed_gas_pool {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The GovernedGasPool resource must exist at the aptos_framework address after initialization.
    /// Criticality: Critical
    /// Implementation: The initialize function ensures the resource is created at the aptos_framework address.
    /// Enforcement: Formally verified via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: Only the aptos_framework address is allowed to initialize the GovernedGasPool.
    /// Criticality: Critical
    /// Implementation: The initialize function verifies the signer is the aptos_framework address.
    /// Enforcement: Formally verified via [high-level-req-2](initialize).
    ///
    /// No.: 3
    /// Requirement: Deposits into the GovernedGasPool must be reflected in the pool's balance.
    /// Criticality: High
    /// Implementation: The deposit and deposit_from functions update the pool's balance.
    /// Enforcement: Formally verified via [high-level-req-3](deposit), [high-level-req-3.1](deposit_from).
    ///
    /// No.: 4
    /// Requirement: Only the aptos_framework address can fund accounts from the GovernedGasPool.
    /// Criticality: High
    /// Implementation: The fund function verifies the signer is the aptos_framework address.
    /// Enforcement: Formally verified via [high-level-req-4](fund).
    ///
    /// No.: 5
    /// Requirement: Gas fees must be deposited into the GovernedGasPool whenever specified by the configuration.
    /// Criticality: High
    /// Implementation: The deposit_gas_fee function ensures gas fees are deposited correctly.
    /// Enforcement: Formally verified via [high-level-req-5](deposit_gas_fee).
    /// </high-level-req>

    spec module {
        /// [high-level-req-1]
        /// The GovernedGasPool resource must exist at aptos_framework after initialization.
        invariant exists<GovernedGasPool>(@aptos_framework);
    }

    /// ensure the aptos_framework signer is used.
    /// aborts if GovernedGasPool already exists.
    spec initialize(aptos_framework: &signer, delegation_pool_creation_seed: vector<u8>) {
        /// [high-level-req-1]
        ensures exists<GovernedGasPool>(@aptos_framework);
        /// [high-level-req-2]
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
    }

    /// Ensure only aptos_framework can fund accounts and balances are updated correctly.
    /// aborts if signer is not aptos_framework.
    spec fund<CoinType>(aptos_framework: &signer, account: address, amount: u64) acquires GovernedGasPool {
        /// [high-level-req-4]
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        ensures coin::balance<CoinType>(account) == old(coin::balance<CoinType>(account)) + amount;
        ensures coin::balance<CoinType>(governed_gas_pool_address()) == 
            old(coin::balance<CoinType>(governed_gas_pool_address())) - amount;
    }

    /// Ensure deposits correctly update the GovernedGasPool balance.
    spec deposit<CoinType>(coin: Coin<CoinType>) acquires GovernedGasPool {
        /// [high-level-req-3]
        ensures coin::balance<CoinType>(governed_gas_pool_address()) == 
            old(coin::balance<CoinType>(governed_gas_pool_address())) + coin.value;
    }

    /// Ensure gas fees are deposited into the GovernedGasPool.
    spec deposit_gas_fee(gas_payer: address, gas_fee: u64) acquires GovernedGasPool {
        /// [high-level-req-5]
        ensures coin::balance<AptosCoin>(governed_gas_pool_address()) == 
            old(coin::balance<AptosCoin>(governed_gas_pool_address())) + gas_fee;
        ensures coin::balance<AptosCoin>(gas_payer) == 
            old(coin::balance<AptosCoin>(gas_payer)) - gas_fee;
    }
}

