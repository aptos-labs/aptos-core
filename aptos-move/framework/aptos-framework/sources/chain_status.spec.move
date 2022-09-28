spec aptos_framework::chain_status {
    spec set_genesis_end {
        pragma verify = false;
    }

    spec schema RequiresIsOperating {
        requires is_operating();
    }
}
