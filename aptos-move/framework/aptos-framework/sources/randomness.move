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
        seed: Option<vector<u8>>,
    }

    public(friend) fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        move_to(framework, PerBlockRandomness {
            seed: option::none(),
        });
    }

    /// Invoked in block prologues to update the block-level randomness seed.
    public(friend) fun on_new_block(vm: &signer, seed_for_new_block: Option<vector<u8>>) acquires PerBlockRandomness {
        system_addresses::assert_vm(vm);
        let seed_holder = borrow_global_mut<PerBlockRandomness>(@aptos_framework);
        seed_holder.seed = seed_for_new_block;
    }
}
