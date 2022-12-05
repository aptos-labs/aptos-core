spec aptos_framework::chain_status {
    spec module {
        pragma verify = false;
        pragma aborts_if_is_strict;
    }

    spec set_genesis_end {
        pragma verify = false;
    }

    spec schema RequiresIsOperating {
        requires is_operating();
    }

    spec assert_operating {
        aborts_if !exists<GenesisEndMarker>(@aptos_framework);
    }

    spec assert_genesis {
        aborts_if exists<GenesisEndMarker>(@aptos_framework);
    }
}
