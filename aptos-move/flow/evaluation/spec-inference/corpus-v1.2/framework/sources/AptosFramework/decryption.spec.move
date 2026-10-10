spec aptos_framework::decryption {
    spec module {
        pragma verify = true;
    }

    spec PerBlockDecryptionKeyV2 {
        invariant option::is_some(decryption_key) == option::is_some(decryption_round);
    }

    spec on_new_block(
        vm: &signer,
        epoch: u64,
        round: u64,
        decryption_key_for_new_block: Option<vector<u8>>
    ) {
        use std::signer;
        pragma opaque;
        modifies global<PerBlockDecryptionKey>(@aptos_framework);
        aborts_if signer::address_of(vm) != @vm;
        ensures exists<PerBlockDecryptionKey>(@aptos_framework) ==>
            global<PerBlockDecryptionKey>(@aptos_framework).decryption_key == decryption_key_for_new_block;
        ensures exists<PerBlockDecryptionKey>(@aptos_framework) ==>
            global<PerBlockDecryptionKey>(@aptos_framework).epoch == epoch;
        ensures exists<PerBlockDecryptionKey>(@aptos_framework) ==>
            global<PerBlockDecryptionKey>(@aptos_framework).round == round;
        ensures !exists<PerBlockDecryptionKey>(@aptos_framework) ==>
            !exists<PerBlockDecryptionKey>(@aptos_framework);
    }

    spec on_new_block_v2(
        vm: &signer,
        epoch: u64,
        round: u64,
        decryption_key_for_new_block: Option<vector<u8>>,
        decryption_round: Option<u64>
    ) {
        use std::signer;
        pragma opaque;
        // Prologue always sends key and round paired (both Some or both None);
        // required to uphold PerBlockDecryptionKeyV2's data invariant.
        requires option::is_some(decryption_key_for_new_block) == option::is_some(decryption_round);
        modifies global<PerBlockDecryptionKeyV2>(@aptos_framework);
        aborts_if signer::address_of(vm) != @vm;
        // next_decryption_round bump is conditional: only when decryption_round is Some.
        aborts_if exists<PerBlockDecryptionKeyV2>(@aptos_framework)
            && option::is_some(decryption_round)
            && option::borrow(decryption_round) + 1 > MAX_U64;
        ensures exists<PerBlockDecryptionKeyV2>(@aptos_framework) ==>
            global<PerBlockDecryptionKeyV2>(@aptos_framework).epoch == epoch;
        ensures exists<PerBlockDecryptionKeyV2>(@aptos_framework) ==>
            global<PerBlockDecryptionKeyV2>(@aptos_framework).block_round == round;
        ensures exists<PerBlockDecryptionKeyV2>(@aptos_framework) ==>
            global<PerBlockDecryptionKeyV2>(@aptos_framework).decryption_key == decryption_key_for_new_block;
        ensures exists<PerBlockDecryptionKeyV2>(@aptos_framework) ==>
            global<PerBlockDecryptionKeyV2>(@aptos_framework).decryption_round == decryption_round;
        ensures (exists<PerBlockDecryptionKeyV2>(@aptos_framework) && option::is_some(decryption_round)) ==>
            global<PerBlockDecryptionKeyV2>(@aptos_framework).next_decryption_round == option::borrow(decryption_round) + 1;
        ensures (exists<PerBlockDecryptionKeyV2>(@aptos_framework) && option::is_none(decryption_round)) ==>
            global<PerBlockDecryptionKeyV2>(@aptos_framework).next_decryption_round
                == old(global<PerBlockDecryptionKeyV2>(@aptos_framework).next_decryption_round);
        ensures !exists<PerBlockDecryptionKeyV2>(@aptos_framework) ==>
            !exists<PerBlockDecryptionKeyV2>(@aptos_framework);
    }

    spec on_new_epoch(framework: &signer, new_epoch: u64) {
        use std::signer;
        use aptos_std::type_info;
        use aptos_std::from_bcs;
        use aptos_std::simple_map;
        use aptos_framework::config_buffer;
        pragma opaque;
        modifies global<PerEpochEncryptionKey>(@aptos_framework);
        modifies global<PerBlockDecryptionKeyV2>(@aptos_framework);
        modifies global<config_buffer::PendingConfigs>(@aptos_framework);
        aborts_if signer::address_of(framework) != @aptos_framework;
        let key = type_info::type_name<PerEpochEncryptionKey>();
        let configs = global<config_buffer::PendingConfigs>(@aptos_framework).configs;
        let stored = simple_map::spec_get(configs, key);
        let pre_does_exist = exists<config_buffer::PendingConfigs>(@aptos_framework)
            && simple_map::spec_contains_key(configs, key);
        aborts_if pre_does_exist && key != stored.type_name;
        aborts_if pre_does_exist && !from_bcs::deserializable<PerEpochEncryptionKey>(stored.data);
        // Buffered key was installed and buffer entry removed.
        ensures pre_does_exist ==> exists<PerEpochEncryptionKey>(@aptos_framework);
        ensures pre_does_exist ==>
            global<PerEpochEncryptionKey>(@aptos_framework)
                == from_bcs::deserialize<PerEpochEncryptionKey>(stored.data);
        let post post_configs = global<config_buffer::PendingConfigs>(@aptos_framework).configs;
        ensures pre_does_exist ==> !simple_map::spec_contains_key(post_configs, key);
        // No buffered key: PerEpochEncryptionKey untouched.
        ensures !pre_does_exist ==>
            (exists<PerEpochEncryptionKey>(@aptos_framework)
                == old(exists<PerEpochEncryptionKey>(@aptos_framework)));
        ensures !pre_does_exist && old(exists<PerEpochEncryptionKey>(@aptos_framework)) ==>
            global<PerEpochEncryptionKey>(@aptos_framework)
                == old(global<PerEpochEncryptionKey>(@aptos_framework));
        // Fresh PerBlockDecryptionKeyV2.
        ensures exists<PerBlockDecryptionKeyV2>(@aptos_framework);
        let post r = global<PerBlockDecryptionKeyV2>(@aptos_framework);
        ensures r.epoch == new_epoch;
        ensures r.block_round == 0;
        ensures option::is_none(r.decryption_key);
        ensures option::is_none(r.decryption_round);
        ensures r.next_decryption_round == 0;
    }
}
