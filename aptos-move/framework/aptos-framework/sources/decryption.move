/// This module provides a decryption key unique to every block. This resource
/// is updated in every block prologue. The decryption key is the key used to
/// decrypt the encrypted transactions in the block.
module aptos_framework::decryption {
    use std::option;
    use std::option::Option;

    use aptos_framework::config_buffer;
    use aptos_framework::system_addresses;

    friend aptos_framework::block;
    friend aptos_framework::reconfiguration_with_dkg;

    /// Decryption key unique to every block.
    /// This resource is updated in every block prologue.
    struct PerBlockDecryptionKey has drop, key {
        epoch: u64,
        round: u64,
        decryption_key: Option<vector<u8>>
    }

    /// Encryption key derived from the DKG result, valid for one epoch.
    struct PerEpochEncryptionKey has drop, key, store {
        epoch: u64,
        encryption_key: Option<vector<u8>>
    }

    /// Called during genesis initialization.
    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<PerBlockDecryptionKey>(@aptos_framework)) {
            move_to(
                framework,
                PerBlockDecryptionKey { epoch: 0, round: 0, decryption_key: option::none() }
            );
        };
        if (!exists<PerEpochEncryptionKey>(@aptos_framework)) {
            move_to(
                framework,
                PerEpochEncryptionKey { epoch: 0, encryption_key: option::none() }
            );
        }
    }

    /// Invoked in block prologues to update the block decryption key.
    public(friend) fun on_new_block(
        vm: &signer,
        epoch: u64,
        round: u64,
        decryption_key_for_new_block: Option<vector<u8>>
    ) acquires PerBlockDecryptionKey {
        system_addresses::assert_vm(vm);
        if (exists<PerBlockDecryptionKey>(@aptos_framework)) {
            let decryption_key =
                borrow_global_mut<PerBlockDecryptionKey>(@aptos_framework);
            decryption_key.epoch = epoch;
            decryption_key.round = round;
            decryption_key.decryption_key = decryption_key_for_new_block;
        }
    }

    /// Buffer the encryption key for the next epoch.
    public(friend) fun set_for_next_epoch(epoch: u64, encryption_key: vector<u8>) {
        config_buffer::upsert(PerEpochEncryptionKey {
            epoch,
            encryption_key: option::some(encryption_key)
        });
    }

    /// Apply buffered PerEpochEncryptionKey on epoch transition.
    public(friend) fun on_new_epoch(framework: &signer) acquires PerEpochEncryptionKey {
        system_addresses::assert_aptos_framework(framework);
        if (config_buffer::does_exist<PerEpochEncryptionKey>()) {
            let new_key = config_buffer::extract_v2<PerEpochEncryptionKey>();
            if (exists<PerEpochEncryptionKey>(@aptos_framework)) {
                *borrow_global_mut<PerEpochEncryptionKey>(@aptos_framework) = new_key;
            } else {
                move_to(framework, new_key);
            }
        }
    }
}
