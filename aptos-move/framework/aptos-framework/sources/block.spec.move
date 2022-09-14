spec aptos_framework::block {
    spec block_prologue {
        use aptos_framework::chain_status;
        requires chain_status::is_operating();
    }
}
