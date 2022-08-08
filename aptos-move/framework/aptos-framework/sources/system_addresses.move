module aptos_framework::system_addresses {
    use std::error;
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

    public fun assert_core_resource_address(addr: address) {
        assert!(is_core_resource_address(addr), error::permission_denied(ENOT_CORE_RESOURCE_ADDRESS))
    }

    public fun is_core_resource_address(addr: address): bool {
        addr == @core_resources
    }

    public fun assert_aptos_framework(account: &signer) {
        assert!(signer::address_of(account) == @aptos_framework, error::permission_denied(ENOT_CORE_FRAMEWORK_ADDRESS))
    }

    /// Assert that the signer has the VM reserved address.
    public fun assert_vm(account: &signer) {
        assert!(signer::address_of(account) == @vm_reserved, error::permission_denied(EVM))
    }
}
