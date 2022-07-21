module aptos_framework::system_addresses {
    use std::errors;
    use std::signer;

    /// The address/account did not correspond to the core resource address
    const ENOT_CORE_RESOURCE_ADDRESS: u64 = 0;
    /// The operation can only be performed by the VM
    const EVM: u64 = 1;
    /// The address/account did not correspond to the core framework address
    const ENOT_CORE_FRAMEWORK_ADDRESS: u64 = 2;

    public fun assert_core_resource(account: &signer) {
        assert_core_resource_address(signer::address_of(account))
    }
    spec assert_core_resource {
        pragma opaque;
        include AbortsIfNotCoreResource {addr: signer::address_of(account) };
    }

    public fun assert_core_resource_address(addr: address) {
        assert!(is_core_resource_address(addr), errors::requires_address(ENOT_CORE_RESOURCE_ADDRESS))
    }
    spec assert_core_resource_address {
        pragma opaque;
        include AbortsIfNotCoreResource;
    }

    /// Specifies that a function aborts if the account does not have the root address.
    spec schema AbortsIfNotCoreResource {
        addr: address;
        aborts_if addr != @core_resources with errors::REQUIRES_ADDRESS;
    }

    public fun is_core_resource_address(addr: address): bool {
        addr == @core_resources
    }

    public fun assert_aptos_framework(account: &signer) {
        assert!(signer::address_of(account) == @aptos_framework, errors::requires_address(ENOT_CORE_FRAMEWORK_ADDRESS))
    }
    spec assert_aptos_framework {
        pragma opaque;
        include AbortsIfNotAptosFramework;
    }

    /// Specifies that a function aborts if the account does not have the aptos framework address.
    spec schema AbortsIfNotAptosFramework {
        account: signer;
        aborts_if signer::address_of(account) != @aptos_framework with errors::REQUIRES_ADDRESS;
    }

    /// Assert that the signer has the VM reserved address.
    public fun assert_vm(account: &signer) {
        assert!(signer::address_of(account) == @vm_reserved, errors::requires_address(EVM))
    }
    spec assert_vm {
        pragma opaque;
        include AbortsIfNotVM;
    }

    /// Specifies that a function aborts if the account does not have the VM reserved address.
    spec schema AbortsIfNotVM {
        account: signer;
        aborts_if signer::address_of(account) != @vm_reserved with errors::REQUIRES_ADDRESS;
    }
}
