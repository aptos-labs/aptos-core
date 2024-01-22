/// On-chain randomness utils.
module aptos_std::randomness {
    use std::option;
    use std::option::Option;
    use aptos_framework::system_addresses;

    friend aptos_framework::block;
    friend aptos_framework::genesis;

    /// Per-block randomness seed.
    /// This resource is updated in every block prologue.
    struct PerBlockRandomness has drop, key {
        epoch: u64,
        round: u64,
        seed: Option<vector<u8>>,
    }

    public(friend) fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        move_to(framework, PerBlockRandomness {
            epoch: 0,
            round: 0,
            seed: option::none(),
        });
    }

    /// Invoked in block prologues to update the block-level randomness seed.
    public(friend) fun on_new_block(vm: &signer, epoch: u64, round: u64, seed_for_new_block: Option<vector<u8>>) acquires PerBlockRandomness {
        system_addresses::assert_vm(vm);
        let randomness = borrow_global_mut<PerBlockRandomness>(@aptos_framework);
        randomness.epoch = epoch;
        randomness.round = round;
        randomness.seed = seed_for_new_block;
    }
}
