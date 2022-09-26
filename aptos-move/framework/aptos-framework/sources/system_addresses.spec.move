spec aptos_framework::system_addresses {
    spec assert_core_resource {
        pragma opaque;
        include AbortsIfNotCoreResource { addr: signer::address_of(account) };
    }

    spec assert_core_resource_address {
        pragma opaque;
        include AbortsIfNotCoreResource;
    }

    /// Specifies that a function aborts if the account does not have the root address.
    spec schema AbortsIfNotCoreResource {
        addr: address;
        aborts_if addr != @core_resources with error::PERMISSION_DENIED;
    }

    spec assert_aptos_framework {
        pragma opaque;
        include AbortsIfNotAptosFramework;
    }

    /// Specifies that a function aborts if the account does not have the aptos framework address.
    spec schema AbortsIfNotAptosFramework {
        account: signer;
        aborts_if signer::address_of(account) != @aptos_framework with error::PERMISSION_DENIED;
    }

    spec assert_vm {
        pragma opaque;
        include AbortsIfNotVM;
    }

    /// Specifies that a function aborts if the account does not have the VM reserved address.
    spec schema AbortsIfNotVM {
        account: signer;
        aborts_if signer::address_of(account) != @vm_reserved with error::PERMISSION_DENIED;
    }
}
