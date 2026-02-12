/// This module provides a decryption key unique to every block. This resource
/// is updated in every block prologue. The decryption key is the key used to
/// decrypt the encrypted transactions in the block.
module aptos_framework::decryption {
    use std::option;
    use std::option::Option;

    use aptos_framework::system_addresses;

    friend aptos_framework::block;

    /// Decryption key unique to every block.
    /// This resource is updated in every block prologue.
    struct PerBlockDecryptionKey has drop, key {
        epoch: u64,
        round: u64,
        decryption_key: Option<vector<u8>>
    }

    /// Called during genesis initialization.
    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<PerBlockDecryptionKey>(@aptos_framework)) {
            move_to(
                framework,
                PerBlockDecryptionKey { epoch: 0, round: 0, decryption_key: option::none() }
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
}
