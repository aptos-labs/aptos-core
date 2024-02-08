spec aptos_std::randomness {

    spec initialize(framework: &signer) {
        use std::option;
        use std::signer;
        let framework_addr = signer::address_of(framework);
        aborts_if framework_addr != @aptos_framework;
        aborts_if exists<PerBlockRandomness>(framework_addr);
        ensures global<PerBlockRandomness>(framework_addr).seed == option::spec_none<vector<u8>>();
    }

    spec on_new_block(vm: &signer, seed_for_new_block: Option<vector<u8>>) {
        use std::signer;
        aborts_if signer::address_of(vm) != @vm;
        aborts_if !exists<PerBlockRandomness>(@aptos_framework);
        ensures global<PerBlockRandomness>(@aptos_framework).seed == seed_for_new_block;
    }

}
