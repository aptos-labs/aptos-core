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
    ///
    /// Superseded by `PerBlockDecryptionKeyV2`: once that resource exists,
    /// blocks run `block_prologue_ext_v3` and only V2 is updated, so this
    /// resource freezes at the last block of the legacy mode. Kept for
    /// chains that predate the upgrade (testnet has committed blocks
    /// updating it via `block_prologue_ext_v2`).
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

    /// `PerBlockDecryptionKey` plus the dense decryption-round tracking that
    /// decouples trusted-setup capacity from the consensus round.
    ///
    /// Its existence marks that round tracking is active: validators emit
    /// `BlockMetadataExt::V3` (updating this resource via
    /// `block_prologue_ext_v3`) iff the resource exists at the epoch root.
    /// Created at genesis on fresh chains and lazily in `on_new_epoch` on
    /// upgraded chains.
    ///
    /// Invariant: `decryption_key.is_some() == decryption_round.is_some()` —
    /// both describe the current block and arrive paired from the prologue.
    struct PerBlockDecryptionKeyV2 has drop, key {
        epoch: u64,
        /// Current block's consensus round.
        block_round: u64,
        /// Current block's decryption key. `none()` when the block carried
        /// no encrypted transactions (or decryption failed).
        decryption_key: Option<vector<u8>>,
        /// Current block's decryption round. `none()` when the block did not
        /// produce a key.
        decryption_round: Option<u64>,
        /// Next decryption round any future key-producing block will consume.
        /// Bumped by one each time a key block hits `on_new_block_v2`. Reset
        /// to 0 on epoch boundaries.
        next_decryption_round: u64
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
        };
        if (!exists<PerBlockDecryptionKeyV2>(@aptos_framework)) {
            move_to(
                framework,
                PerBlockDecryptionKeyV2 {
                    epoch: 0,
                    block_round: 0,
                    decryption_key: option::none(),
                    decryption_round: option::none(),
                    next_decryption_round: 0
                }
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

    /// Invoked in `block_prologue_ext_v3`. `block_round` advances every
    /// block; `next_decryption_round` is sticky and only bumps when the
    /// pipeline sends a key (paired with the round it consumed).
    public(friend) fun on_new_block_v2(
        vm: &signer,
        epoch: u64,
        round: u64,
        decryption_key_for_new_block: Option<vector<u8>>,
        decryption_round: Option<u64>
    ) acquires PerBlockDecryptionKeyV2 {
        system_addresses::assert_vm(vm);
        if (exists<PerBlockDecryptionKeyV2>(@aptos_framework)) {
            let r = borrow_global_mut<PerBlockDecryptionKeyV2>(@aptos_framework);
            r.epoch = epoch;
            r.block_round = round;
            r.decryption_key = decryption_key_for_new_block;
            r.decryption_round = decryption_round;
            if (option::is_some(&decryption_round)) {
                r.next_decryption_round = *option::borrow(&decryption_round) + 1;
            }
        }
    }

    /// Buffer the encryption key for the next epoch.
    public(friend) fun set_for_next_epoch(epoch: u64, encryption_key: vector<u8>) {
        config_buffer::upsert(PerEpochEncryptionKey {
            epoch,
            encryption_key: option::some(encryption_key)
        });
    }

    /// Apply buffered PerEpochEncryptionKey and reset PerBlockDecryptionKeyV2,
    /// creating the latter on chains that predate it.
    public(friend) fun on_new_epoch(
        framework: &signer, new_epoch: u64
    ) acquires PerEpochEncryptionKey, PerBlockDecryptionKeyV2 {
        system_addresses::assert_aptos_framework(framework);
        if (config_buffer::does_exist<PerEpochEncryptionKey>()) {
            let new_key = config_buffer::extract_v2<PerEpochEncryptionKey>();
            if (exists<PerEpochEncryptionKey>(@aptos_framework)) {
                *borrow_global_mut<PerEpochEncryptionKey>(@aptos_framework) = new_key;
            } else {
                move_to(framework, new_key);
            }
        };
        let fresh = PerBlockDecryptionKeyV2 {
            epoch: new_epoch,
            block_round: 0,
            decryption_key: option::none(),
            decryption_round: option::none(),
            next_decryption_round: 0
        };
        if (exists<PerBlockDecryptionKeyV2>(@aptos_framework)) {
            *borrow_global_mut<PerBlockDecryptionKeyV2>(@aptos_framework) = fresh;
        } else {
            move_to(framework, fresh);
        }
    }
}
