spec aptos_framework::system_addresses {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec assert_core_resource(account: &signer) {
        pragma opaque;
        include AbortsIfNotCoreResource { addr: signer::address_of(account) };
    }

    spec assert_core_resource_address(addr: address) {
        pragma opaque;
        include AbortsIfNotCoreResource;
    }

    spec is_core_resource_address(addr: address): bool {
        pragma opaque;
        aborts_if false;
        ensures result == (addr == @core_resources);
    }

    /// Specifies that a function aborts if the account does not have the root address.
    spec schema AbortsIfNotCoreResource {
        addr: address;
        aborts_if addr != @core_resources with error::PERMISSION_DENIED;
    }

    spec assert_aptos_framework(account: &signer) {
        pragma opaque;
        include AbortsIfNotAptosFramework;
    }

    spec assert_framework_reserved_address(account: &signer) {
        aborts_if !is_framework_reserved_address(signer::address_of(account));
    }

    spec assert_framework_reserved(addr: address) {
        aborts_if !is_framework_reserved_address(addr);
    }
    /// Specifies that a function aborts if the account does not have the aptos framework address.
    spec schema AbortsIfNotAptosFramework {
        account: signer;
        aborts_if signer::address_of(account) != @aptos_framework with error::PERMISSION_DENIED;
    }

    spec assert_vm(account: &signer) {
        pragma opaque;
        include AbortsIfNotVM;
    }

    /// Specifies that a function aborts if the account does not have the VM reserved address.
    spec schema AbortsIfNotVM {
        account: signer;
        aborts_if signer::address_of(account) != @vm_reserved with error::PERMISSION_DENIED;
    }
}
