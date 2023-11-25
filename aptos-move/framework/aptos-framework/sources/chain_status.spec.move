spec aptos_framework::chain_status {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec set_genesis_end(aptos_framework: &signer) {
        use std::signer;
        //TODO: It occured gloabl invariants not hold (property proved)
        pragma verify = false;
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if exists<GenesisEndMarker>(@aptos_framework);
        ensures global<GenesisEndMarker>(@aptos_framework) == GenesisEndMarker {};
    }

    spec schema RequiresIsOperating {
        requires is_operating();
    }

    spec assert_operating {
        aborts_if !is_operating();
    }

    spec assert_genesis {
        aborts_if !is_genesis();
    }
}
