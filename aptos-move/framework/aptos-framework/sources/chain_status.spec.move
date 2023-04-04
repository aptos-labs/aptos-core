spec aptos_framework::chain_status {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec set_genesis_end {
        pragma verify = false;
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
