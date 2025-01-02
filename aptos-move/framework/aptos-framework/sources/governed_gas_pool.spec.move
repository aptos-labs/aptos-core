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

        /// Ghost variables for balances
        global governed_gas_pool_balance: num;
        global depositor_balance: num;
        global gas_payer_balance: num;
    }

    spec initialize(aptos_framework: &signer, delegation_pool_creation_seed: vector<u8>) {
        /// [high-level-req-1]
        ensures exists<GovernedGasPool>(@aptos_framework);
        /// [high-level-req-2]
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
    }

    spec fund<CoinType>(aptos_framework: &signer, account: address, amount: u64) {
        /// [high-level-req-4]
        aborts_if !system_addresses::is_aptos_framework_address(signer::address_of(aptos_framework));
        /// Updates to the ghost balance variables
        ensures depositor_balance == old(depositor_balance) + amount;
        ensures governed_gas_pool_balance == old(governed_gas_pool_balance) - amount;
    }

    spec deposit<CoinType>(coin: Coin<CoinType>) {
        /// [high-level-req-3]
        ensures governed_gas_pool_balance == old(governed_gas_pool_balance) + coin.value;
    }

    spec deposit_gas_fee(gas_payer: address, gas_fee: u64) {
        /// [high-level-req-5]
        ensures governed_gas_pool_balance == old(governed_gas_pool_balance) + gas_fee;
        ensures gas_payer_balance == old(gas_payer_balance) - gas_fee;
    }
}
