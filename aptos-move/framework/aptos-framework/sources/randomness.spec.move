spec aptos_std::randomness {
    spec fetch_and_increment_txn_counter(): vector<u8> {
        pragma opaque;
    }

    spec is_safe_call(): bool {
        pragma opaque;
    }
}
