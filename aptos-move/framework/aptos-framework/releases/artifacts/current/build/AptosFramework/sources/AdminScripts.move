module AptosFramework::AdminScripts {
    use AptosFramework::TestCoin;
    use AptosFramework::AptosVMConfig;

    public(script) fun delegate_mint_capability(core_resource_account: signer, addr: address) {
        TestCoin::delegate_mint_capability(&core_resource_account, addr);
    }

    public(script) fun claim_mint_capability(sender: signer) {
        TestCoin::claim_mint_capability(&sender);
    }

    public(script) fun mint(sender: signer, addr: address, amount: u64) {
        TestCoin::mint(&sender, addr, amount);
    }

    public(script) fun set_gas_constants(
        sender: signer,
        global_memory_per_byte_cost: u64,
        global_memory_per_byte_write_cost: u64,
        min_transaction_gas_units: u64,
        large_transaction_cutoff: u64,
        intrinsic_gas_per_byte: u64,
        maximum_number_of_gas_units: u64,
        min_price_per_gas_unit: u64,
        max_price_per_gas_unit: u64,
        max_transaction_size_in_bytes: u64,
        gas_unit_scaling_factor: u64,
        default_account_size: u64,
    ) {
        AptosVMConfig::set_gas_constants(
            &sender,
            global_memory_per_byte_cost,
            global_memory_per_byte_write_cost,
            min_transaction_gas_units,
            large_transaction_cutoff,
            intrinsic_gas_per_byte,
            maximum_number_of_gas_units,
            min_price_per_gas_unit,
            max_price_per_gas_unit,
            max_transaction_size_in_bytes,
            gas_unit_scaling_factor,
            default_account_size,
        );
    }
}
