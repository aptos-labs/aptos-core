module message_transmitter::message_transmitter {
    use message_transmitter::state;

    const SEED: vector<u8> = b"SEED";

    fun reserve_and_increment_nonce(): u64 {
        let nonce = state::get_next_available_nonce();
        state::set_next_available_nonce(nonce+1);
        nonce
    }
}

