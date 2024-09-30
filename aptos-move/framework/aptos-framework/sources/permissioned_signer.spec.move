spec aptos_framework::permissioned_signer {
    spec module {
        pragma verify = false;
    }

    spec fun spec_is_permissioned_signer(s: signer): bool;

    spec is_permissioned_signer(s: &signer): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_permissioned_signer(s);
    }

    spec fun spec_permission_signer(s: signer): signer;

    spec permission_signer(permissioned: &signer): signer {
        pragma opaque;
        ensures [abstract] result == spec_permission_signer(permissioned);
    }


}
