module AptosFramework::Marker {
    use Std::Capability;
    use CoreFramework::DiemTimestamp;
    use CoreFramework::SystemAddresses;

    friend AptosFramework::AptosAccount;
    friend AptosFramework::AptosConsensusConfig;
    friend AptosFramework::AptosValidatorConfig;
    friend AptosFramework::AptosValidatorOperatorConfig;
    friend AptosFramework::AptosValidatorSet;
    friend AptosFramework::AptosVersion;
    friend AptosFramework::AptosVMConfig;

    struct ChainMarker has drop {}

    public(friend) fun get(): ChainMarker {
        ChainMarker {}
    }

    /// Initialize the capability of the marker so friend modules can acquire it for priviledged operations.
    public fun initialize(core_resource: &signer) {
        DiemTimestamp::assert_genesis();
        SystemAddresses::assert_core_resource(core_resource);
        Capability::create(core_resource, &get());
    }


    #[test(account = @0x42)]
    #[expected_failure]
    fun initialize_with_wrong_addr(account: signer) {
        initialize(&account)
    }
}
