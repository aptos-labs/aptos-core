/// This module provides a veiled coin type, denoted `VeiledCoin<T>` that hides the value/denomination of a coin.
///
/// Our implementation relies on a secondary so-called "resource account" which helps us mint a `VeiledCoin<T>` from a
/// traditional `coin::Coin<T>` by transferring this latter coin into a `coin::CoinStore<T>` resource stored in the
/// resource account. Later on, when someone wants to convert their `VeiledCoin<T>` into a traditional `coin::Coin<T>`
/// the resource account can be used to transfer out said `coin::Coin<T>` from its coin store. This is where the
/// "resource account" becomes important, since transfering out a coin like this requires a `signer` for the resource
/// account, which this module can obtain via a `SignerCapability`.
module veiled_coin::veiled_coin {
    use std::error;
    use std::signer;
    use std::features;
    use std::vector;
    use std::option::Option;
    use aptos_std::elgamal::{Self, Ciphertext, CompressedCiphertext, Pubkey};
    use aptos_std::ristretto255::{RistrettoPoint, Self, Scalar, new_scalar_from_u64};
    //use aptos_std::debug::print;

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::bulletproofs::RangeProof;
    use aptos_std::bulletproofs;

    //
    // Errors.
    //

    /// The range proof system needs to support proofs for any number \in [0, 2^{32})
    const ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE : u64 = 1;

    /// A range proof failed to verify.
    const ERANGE_PROOF_VERIFICATION_FAILED : u64 = 2;

    /// Account already has `VeiledCoinStore` registered for `CoinType`
    const EVEILED_COIN_STORE_ALREADY_PUBLISHED: u64 = 4;

    /// Account hasn't registered `VeiledCoinStore` for `CoinType`
    const EVEILED_COIN_STORE_NOT_PUBLISHED: u64 = 5;

    /// Not enough coins to complete transaction
    const EINSUFFICIENT_BALANCE: u64 = 6;

    /// Byte vector failed to deserialize to ciphertexts
    const EDESERIALIZATION_FAILED: u64 = 7;

    /// Byte vector given for deserialization was the wrong length
    const EBYTES_WRONG_LENGTH: u64 = 8;

    /// Sigma protocol for withdrawals failed
    const ESIGMA_PROTOCOL_VERIFY_FAILED: u64 = 9;

    /// Ciphertext has wrong value when unwrapping
    const ECIPHERTEXT_WRONG_VALUE: u64 = 10;

    //
    // Constants
    //

    /// The maximum number of bits used to represent a coin's value.
    const MAX_BITS_IN_VALUE : u64 = 32;

    /// The domain separation tag (DST) used for the Bulletproofs prover.
    const VEILED_COIN_DST : vector<u8> = b"AptosVeiledCoinExample";
    const FIAT_SHAMIR_SIGMA_DST : vector<u8> = b"SigmaFiatShamir";

    //
    // Core data structures.
    //

    // TODO: Describe in comment
    struct VeiledTransferProof<phantom CoinType> has drop {
        updated_balance_proof: RangeProof,
        transferred_amount_proof: RangeProof,
        sigma_proof: SigmaProof<CoinType>,
    }

    // TODO: Describe in comment
    struct SigmaProof<phantom CoinType> has drop {
        x1: RistrettoPoint,
        x2: RistrettoPoint, 
        x3: RistrettoPoint,
        x4: RistrettoPoint,
        x5: RistrettoPoint,
        alpha1: Scalar,
        alpha2: Scalar,
        alpha3: Scalar,
        alpha4: Scalar,
    }

    // TODO: Replace trim with more gas efficient function
    /// Deserializes and returns a VeiledWithdrawalProof given its byte representation
    fun deserialize_sigma_proof<CoinType>(proof_bytes: vector<u8>): Option<SigmaProof<CoinType>> {
        assert!(vector::length<u8>(&proof_bytes) == 288, EBYTES_WRONG_LENGTH);
        let trimmed_bytes_x1 = vector::trim<u8>(&mut proof_bytes, 32);
        let x1 = ristretto255::new_point_from_bytes(proof_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x1)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x1 = std::option::extract<RistrettoPoint>(&mut x1);

        let trimmed_bytes_x2 = vector::trim(&mut trimmed_bytes_x1, 32);    
        let x2 = ristretto255::new_point_from_bytes(trimmed_bytes_x1);
        if (!std::option::is_some<RistrettoPoint>(&x2)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x2 = std::option::extract<RistrettoPoint>(&mut x2);    

        let trimmed_bytes_x3 = vector::trim(&mut trimmed_bytes_x2, 32);    
        let x3 = ristretto255::new_point_from_bytes(trimmed_bytes_x2);
        if (!std::option::is_some<RistrettoPoint>(&x3)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x3 = std::option::extract<RistrettoPoint>(&mut x3);

        let trimmed_bytes_x4 = vector::trim(&mut trimmed_bytes_x3, 32);    
        let x4 = ristretto255::new_point_from_bytes(trimmed_bytes_x3);
        if (!std::option::is_some<RistrettoPoint>(&x4)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x4 = std::option::extract<RistrettoPoint>(&mut x4);

        let trimmed_bytes_x5 = vector::trim(&mut trimmed_bytes_x4, 32);    
        let x5 = ristretto255::new_point_from_bytes(trimmed_bytes_x4);
        if (!std::option::is_some<RistrettoPoint>(&x5)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x5 = std::option::extract<RistrettoPoint>(&mut x5);

        let trimmed_bytes_alpha1 = vector::trim(&mut trimmed_bytes_x5, 32);
        let alpha1 = ristretto255::new_scalar_from_bytes(trimmed_bytes_x5);
        if (!std::option::is_some(&alpha1)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let alpha1 = std::option::extract(&mut alpha1);

        let trimmed_bytes_alpha2 = vector::trim(&mut trimmed_bytes_alpha1, 32);
        let alpha2 = ristretto255::new_scalar_from_bytes(trimmed_bytes_alpha1);
        if (!std::option::is_some(&alpha2)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let alpha2 = std::option::extract(&mut alpha2);

        let trimmed_bytes_alpha3 = vector::trim(&mut trimmed_bytes_alpha2, 32);
        let alpha3 = ristretto255::new_scalar_from_bytes(trimmed_bytes_alpha2);
        if (!std::option::is_some(&alpha3)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let alpha3 = std::option::extract(&mut alpha3);

        let _trimmed_bytes_alpha4 = vector::trim(&mut trimmed_bytes_alpha3, 32);
        let alpha4 = ristretto255::new_scalar_from_bytes(trimmed_bytes_alpha3);
        if (!std::option::is_some(&alpha4)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let alpha4 = std::option::extract(&mut alpha4);

        std::option::some(SigmaProof {
            x1, x2, x3, x4, x5, alpha1, alpha2, alpha3, alpha4
        })
    }

    /// Main structure representing a coin in an account's custody.
    struct VeiledCoin<phantom CoinType> {
        /// ElGamal ciphertext of a number of coins v \in [0, 2^{32}), an invariant
        /// that is enforced throughout the code
        private_value: Ciphertext,
    }

    /// A holder of a specific coin type and its associated event handles.
    /// These are kept in a single resource to ensure locality of data.
    struct VeiledCoinStore<phantom CoinType> has key {
        /// A ElGamal ciphertext of a value v \in [0, 2^{32}), an invariant that is enforced throughout the code.
        private_balance: CompressedCiphertext,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
        pubkey: elgamal::Pubkey,
    }

    /// Holds a signer capability for the resource account created when initializing this module. This account houses a
    /// coin::CoinStore<T> for every type of coin T that is veiled.
    struct VeiledCoinMinter has store, key {
        signer_cap: account::SignerCapability,
    }

    /// Event emitted when some amount of a coin is deposited into an account.
    struct DepositEvent has drop, store {
        // We cannot leak any information about how much has been deposited.
    }

    /// Event emitted when some amount of a coin is withdrawn from an account.
    struct WithdrawEvent has drop, store {
        // We cannot leak any information about how much has been withdrawn.
    }

    //
    // Module initialization, done only once when this module is first published on the blockchain
    //

    /// Initializes a so-called "resource" account which will maintain a coin::CoinStore<T> resource for all Coin<T>'s
    /// that have been converted into a VeiledCoin<T>.
    fun init_module(deployer: &signer) {
        assert!(bulletproofs::get_max_range_bits() >= MAX_BITS_IN_VALUE, ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE);

        // Create the resource account. This will allow this module to later obtain a `signer` for this account and
        // transfer Coin<T>'s into its CoinStore<T> before minting a VeiledCoin<T>.
        let (_resource, signer_cap) = account::create_resource_account(deployer, vector::empty());

        move_to(deployer,
            VeiledCoinMinter {
                signer_cap
            }
        )
    }

    //
    // Entry functions
    //

    /// Initializes a veiled coin store for the specified account. Requires an ElGamal public
    /// encryption key to be provided. The user must retain their corresponding secret key. 
    public entry fun register<CoinType>(sender: &signer, pubkey: vector<u8>) {
        let pubkey = elgamal::new_pubkey_from_bytes(pubkey);    
        register_internal<CoinType>(sender, std::option::extract(&mut pubkey));
    }

    /// Takes `amount` of `coin::Coin<CoinType>` coins from `sender`, wraps them inside a `VeiledCoin<CoinType>` and
    /// sends them back to `sender`.
    public entry fun wrap<CoinType>(sender: &signer, amount: u64) acquires VeiledCoinMinter, VeiledCoinStore {
        wrap_to<CoinType>(sender, signer::address_of(sender), amount)
    }

    /// Takes `amount` of `coin::Coin<CoinType>` coins from `sender`, wraps them inside a `VeiledCoin<CoinType>` and
    /// sends to `recipient`. Note that the returned veiled coin will not actually be private yet, since the amount is public here.
    public entry fun wrap_to<CoinType>(sender: &signer, recipient: address, amount: u64) acquires VeiledCoinMinter, VeiledCoinStore {
        let c = coin::withdraw<CoinType>(sender, amount);

        let vc = wrap_from_coin(c);

        deposit<CoinType>(recipient, vc)
    }

    /// Takes `amount` of `VeiledCoin<CoinType>` coins from `sender`, unwraps them to a coin::Coin<CoinType>,
    /// and sends them back to `sender`. Note that this function inherently leaks `amount`. Privacy of the remaining balance is maintained. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send.
    /// `amount` being the proper size is enforce by its being a u32.
    public entry fun unwrap<CoinType>(
        sender: &signer, 
        amount: u32, 
        range_proof_updated_balance: vector<u8>) acquires VeiledCoinStore, VeiledCoinMinter
    {
        unwrap_to<CoinType>(sender, signer::address_of(sender), amount, range_proof_updated_balance)
    }

    /// Takes `amount` of `VeiledCoin<CoinType>` coins from `sender`, unwraps them to a coin::Coin<CoinType>,
    /// and sends them to `recipient`. It is not possible for this function to be made private. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send.
    /// `amount` being the proper size is enforce by its being a u32.
    public entry fun unwrap_to<CoinType>(
        sender: &signer, 
        recipient: address, 
        amount: u32, 
        range_proof_updated_balance_bytes: vector<u8>) acquires VeiledCoinStore, VeiledCoinMinter
    {
        let range_proof_updated_balance = bulletproofs::range_proof_from_bytes(range_proof_updated_balance_bytes);
    
        let c = unwrap_to_coin<CoinType>(sender, amount, &range_proof_updated_balance);
        coin::deposit<CoinType>(recipient, c);
    }

    /// Sends the specified private amount to `recipient` and updates the private balance of `sender`. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a 
    /// range proof on the transferred amount. Also requires a sigma protocol to prove that 
    /// 'private_withdraw_amount' encrypts the same value using the same randomness as 'private_deposit_amount'. 
    /// This value is the amount being transferred. These two ciphertexts are required as we need to update 
    /// both the sender's and the recipient's balances, which use different public keys and so must be updated 
    /// with ciphertexts encrypted with their respective public keys. 
    public entry fun private_transfer_to_entry<CoinType>(
        sender: &signer, 
        recipient: address, 
        withdraw_ct: vector<u8>, 
        deposit_ct: vector<u8>, 
        range_proof_updated_balance_bytes: vector<u8>, 
        range_proof_transferred_amount_bytes: vector<u8>,
        sigma_proof_bytes: vector<u8>) acquires VeiledCoinStore 
    {
        let private_withdraw_amount = elgamal::new_ciphertext_from_bytes(withdraw_ct);
        assert!(std::option::is_some(&private_withdraw_amount), EDESERIALIZATION_FAILED);
        let private_deposit_amount = elgamal::new_ciphertext_from_bytes(deposit_ct);
        assert!(std::option::is_some(&private_deposit_amount), EDESERIALIZATION_FAILED);
        let sigma_proof = deserialize_sigma_proof<CoinType>(sigma_proof_bytes);
        assert!(std::option::is_some(&sigma_proof), EDESERIALIZATION_FAILED);

        let updated_balance_proof = bulletproofs::range_proof_from_bytes(range_proof_updated_balance_bytes);
        let transferred_amount_proof = bulletproofs::range_proof_from_bytes(range_proof_transferred_amount_bytes);

        let transfer_proof = VeiledTransferProof { updated_balance_proof, transferred_amount_proof, sigma_proof: std::option::extract(&mut sigma_proof) };

        private_transfer_to<CoinType>(
            sender,
            recipient,
            std::option::extract(&mut private_withdraw_amount),
            std::option::extract(&mut private_deposit_amount),
            &transfer_proof,
        )
    }


    //
    // Private functions.
    //

    /// Returns a signer for the resource account.
    fun get_resource_account_signer(): signer acquires VeiledCoinMinter {
        account::create_signer_with_capability(&borrow_global<VeiledCoinMinter>(@veiled_coin).signer_cap)
    }

    /// Used internally to drop veiled coins that were split or joined.
    fun drop_veiled_coin<CoinType>(c: VeiledCoin<CoinType>) {
        let VeiledCoin<CoinType> { private_value: _ } = c;
    }

    /// Given an address, returns the public key in the VeiledCoinStore associated with that address
    fun get_pubkey_from_addr<CoinType>(addr: address): Pubkey acquires VeiledCoinStore {
        assert!(has_veiled_coin_store<CoinType>(addr), EVEILED_COIN_STORE_NOT_PUBLISHED);
        let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(addr);
        coin_store.pubkey
    }

    /// Verifies a sigma proof needed to perform a private withdrawal. The relation is described on
    /// page 14 of the Zether paper: https://crypto.stanford.edu/~buenz/papers/zether.pdf
    // TODO: Finish description
    fun verify_withdrawal_sigma_protocol<CoinType>(sender_pubkey: &elgamal::Pubkey, recipient_pubkey: &Pubkey, balance: &Ciphertext, withdrawal_ct: &Ciphertext, deposit_ct: &Ciphertext, proof: &SigmaProof<CoinType>) {
        let sender_pubkey_point = elgamal::get_point_from_pubkey(sender_pubkey);
        let recipient_pubkey_point = elgamal::get_point_from_pubkey(recipient_pubkey);
        let (big_c, d) = elgamal::ciphertext_as_points(withdrawal_ct);
        let (bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let (c_L, c_R) = elgamal::ciphertext_as_points(balance);

        // TODO: Can this be optimized so we don't re-serialize the proof for fiat-shamir?
        // c <- H(g,y,\bar{y},C_L,C_R,C,D,\bar{C},X_1,X_2,X_3,X_4,X_5)
        let hash_input = vector::empty<u8>();

        let basepoint_bytes = ristretto255::point_to_bytes(&ristretto255::basepoint_compressed());
        vector::append<u8>(&mut hash_input, basepoint_bytes);

        let y = elgamal::get_compressed_point_from_pubkey(sender_pubkey);
        let y_bytes = ristretto255::point_to_bytes(&y);
        vector::append<u8>(&mut hash_input, y_bytes);

        let y_bar = elgamal::get_compressed_point_from_pubkey(recipient_pubkey);
        let y_bar_bytes = ristretto255::point_to_bytes(&y_bar);
        vector::append<u8>(&mut hash_input, y_bar_bytes);

        let c_L_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c_L));
        vector::append<u8>(&mut hash_input, c_L_bytes);

        let c_R_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c_R));
        vector::append<u8>(&mut hash_input, c_R_bytes);

        let big_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(big_c));
        vector::append<u8>(&mut hash_input, big_c_bytes);

        let d_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(d));
        vector::append<u8>(&mut hash_input, d_bytes);

        let bar_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(bar_c));
        vector::append<u8>(&mut hash_input, bar_c_bytes);

        let x_1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x1));
        vector::append<u8>(&mut hash_input, x_1_bytes);

        let x_2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x2));
        vector::append<u8>(&mut hash_input, x_2_bytes);

        let x_3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x3));
        vector::append<u8>(&mut hash_input, x_3_bytes);

        let x_4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x4));
        vector::append<u8>(&mut hash_input, x_4_bytes);

        let x_5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x5));
        vector::append<u8>(&mut hash_input, x_5_bytes);

        vector::append<u8>(&mut hash_input, FIAT_SHAMIR_SIGMA_DST);

        let c = ristretto255::new_scalar_from_sha2_512(hash_input);

        // c * D + X1 =? \alpha_1 * g
        let d_acc = ristretto255::point_mul(d, &c);    
        ristretto255::point_add_assign(&mut d_acc, &proof.x1);
        let g_alpha1 = ristretto255::basepoint_mul(&proof.alpha1);
        assert!(ristretto255::point_equals(&d_acc, &g_alpha1), ESIGMA_PROTOCOL_VERIFY_FAILED);

        // c * y + X2 =? \alpha_2 * g
        let y_times_c = ristretto255::point_mul(&sender_pubkey_point, &c);
        ristretto255::point_add_assign(&mut y_times_c, &proof.x2);
        let g_alpha2 = ristretto255::basepoint_mul(&proof.alpha2);
        assert!(ristretto255::point_equals(&y_times_c, &g_alpha2), ESIGMA_PROTOCOL_VERIFY_FAILED);

        let g_alpha3 = ristretto255::basepoint_mul(&proof.alpha3); 
        // c * C + X3 =? \alpha_3 * g + \alpha_1 * y
        let big_c = ristretto255::point_mul(big_c, &c);
        ristretto255::point_add_assign(&mut big_c, &proof.x3);
        let y_alpha1 = ristretto255::point_mul(&sender_pubkey_point, &proof.alpha1);
        ristretto255::point_add_assign(&mut y_alpha1, &g_alpha3);
        assert!(ristretto255::point_equals(&big_c, &y_alpha1), ESIGMA_PROTOCOL_VERIFY_FAILED);

        // c * \bar{C} + X4 =? \alpha_3 * g + \alpha_1 * \bar{y}
        let bar_c = ristretto255::point_mul(bar_c, &c);
        ristretto255::point_add_assign(&mut bar_c, &proof.x4);
        let bar_y_alpha1 = ristretto255::point_mul(&recipient_pubkey_point, &proof.alpha1);
        ristretto255::point_add_assign(&mut bar_y_alpha1, &g_alpha3);
        assert!(ristretto255::point_equals(&bar_c, &bar_y_alpha1), ESIGMA_PROTOCOL_VERIFY_FAILED);

        // c * (C_L + -C) + X5 =? \alpha_4 * g + \alpha_2 * (C_R + -D)
        let neg_C = ristretto255::point_neg(&big_c);
        ristretto255::point_add_assign(&mut neg_C, c_L);
        ristretto255::point_mul_assign(&mut neg_C, &c);
        ristretto255::point_add_assign(&mut neg_C, &proof.x5);
        let neg_D = ristretto255::point_neg(d);
        ristretto255::point_add_assign(&mut neg_D, c_R);
        ristretto255::point_mul_assign(&mut neg_D, &proof.alpha2);
        let g_alpha4 = ristretto255::basepoint_mul(&proof.alpha4);
        ristretto255::point_add_assign(&mut neg_D, &g_alpha4);
        assert!(ristretto255::point_equals(&neg_C, &neg_D), ESIGMA_PROTOCOL_VERIFY_FAILED);
    }

    //
    // Public functions.
    //

    /// Returns `true` if `account_addr` is registered to receive veiled coins of `CoinType`.
    public fun has_veiled_coin_store<CoinType>(account_addr: address): bool {
        exists<VeiledCoinStore<CoinType>>(account_addr)
    }

    /// Returns the ciphertext of the value of `coin`.
    public fun private_value<CoinType>(coin: &VeiledCoin<CoinType>): &Ciphertext {
        &coin.private_value
    }

    /// Initializes a veiled coin store for the specified account. Requires an ElGamal public
    /// encryption key to be provided. The user must retain their corresponding secret key. 
    public fun register_internal<CoinType>(account: &signer, pubkey: elgamal::Pubkey) {
        let account_addr = signer::address_of(account);
        assert!(
            !has_veiled_coin_store<CoinType>(account_addr),
            error::already_exists(EVEILED_COIN_STORE_ALREADY_PUBLISHED),
        );

        let coin_store = VeiledCoinStore<CoinType> {
            private_balance: elgamal::new_ciphertext_from_compressed(ristretto255::point_identity_compressed(), ristretto255::point_identity_compressed()),
            deposit_events: account::new_event_handle<DepositEvent>(account),
            withdraw_events: account::new_event_handle<WithdrawEvent>(account),
            pubkey: pubkey,
        };
        move_to(account, coin_store);
    }

    /// Mints a veiled coin from a normal coin, shelving the normal coin into the resource account's coin store.
    ///
    /// WARNING: Fundamentally, there is no way to hide the value of the coin being minted here.
    public fun wrap_from_coin<CoinType>(c: Coin<CoinType>): VeiledCoin<CoinType> acquires VeiledCoinMinter {
        // If there is no CoinStore<CoinType> yet, create one.
        let rsrc_acc_signer = get_resource_account_signer();
        let rsrc_acc_addr = signer::address_of(&rsrc_acc_signer);
        if (!coin::is_account_registered<CoinType>(rsrc_acc_addr)) {
            coin::register<CoinType>(&rsrc_acc_signer);
        };

        // Move the traditional coin into the coin store, so we can mint a veiled coin.
        // (There is no other way to drop a traditional coin, for safety reasons, so moving it into a coin store is
        //    the only option.)
        let value = new_scalar_from_u64(coin::value(&c));
        coin::deposit(rsrc_acc_addr, c);

        VeiledCoin<CoinType> {
            private_value: elgamal::new_ciphertext_no_randomness(&value)
        }
    }

    /// Returns the ciphertext of the balance of `owner` for the provided `CoinType`.
    public fun private_balance<CoinType>(owner: address): CompressedCiphertext acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(owner),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        borrow_global<VeiledCoinStore<CoinType>>(owner).private_balance
    }

    /// Removes `amount` coins from the VeiledCoinStore balance of `sender`, by subtracting `amount_ct` from
    /// their VeiledCoinStore balance. It is not possible for this function to be made private. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a 
    /// range proof on the transferred amount.
    public fun unwrap_to_coin<CoinType>(
        sender: &signer, 
        amount: u32,
        range_proof_updated_balance: &RangeProof): Coin<CoinType> acquires VeiledCoinStore, VeiledCoinMinter {
    // resource account signer should exist as wrap_to_coin should already have been called
        let rsrc_acc_signer = get_resource_account_signer();
        let scalar_amount = new_scalar_from_u64((amount as u64));
        let computed_ct = elgamal::new_ciphertext_no_randomness(&scalar_amount);

        let sender_addr = signer::address_of(sender);
        let sender_coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(sender_addr);

        withdraw(sender, computed_ct, sender_coin_store, range_proof_updated_balance, &std::option::none());
        coin::withdraw(&rsrc_acc_signer, (amount as u64))
    }

    /// Sends the specified private amount to `recipient` and updates the private balance of `sender`. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a 
    /// range proof on the transferred amount. Also requires a sigma protocol to prove that 
    /// 'private_withdraw_amount' encrypts the same value using the same randomness as 'private_deposit_amount'. 
    /// This value is the amount being transferred. These two ciphertexts are required as we need to update 
    /// both the sender's and the recipient's balances, which use different public keys and so must be updated 
    /// with ciphertexts encrypted with their respective public keys. 
    public fun private_transfer_to<CoinType>(
        sender: &signer,
        recipient_addr: address,
        private_withdraw_amount: Ciphertext,
        private_deposit_amount: Ciphertext,
        transfer_proof: &VeiledTransferProof<CoinType>) acquires VeiledCoinStore 
    {
        let sender_addr = signer::address_of(sender);

        // get_pubkey_from_addr checks if each account has a VeiledCoinStore published
        let sender_pubkey = get_pubkey_from_addr<CoinType>(sender_addr);
        let recipient_pubkey = get_pubkey_from_addr<CoinType>(recipient_addr);
        let sender_coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(sender_addr);

        verify_withdrawal_sigma_protocol(&sender_pubkey, &recipient_pubkey, &elgamal::decompress_ciphertext(&sender_coin_store.private_balance), &private_withdraw_amount, &private_deposit_amount, &transfer_proof.sigma_proof);

        withdraw<CoinType>(sender, private_withdraw_amount, sender_coin_store, &transfer_proof.updated_balance_proof, &std::option::some(transfer_proof.transferred_amount_proof));
        let vc = VeiledCoin<CoinType> { private_value: private_deposit_amount };

        deposit(recipient_addr, vc);
    }

    /// Deposits a veiled coin at address `to_addr`.
    public fun deposit<CoinType>(to_addr: address, coin: VeiledCoin<CoinType>) acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(to_addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(to_addr);

        event::emit_event<DepositEvent>(
            &mut coin_store.deposit_events,
            DepositEvent {},
        );

        let private_balance = elgamal::decompress_ciphertext(&coin_store.private_balance);

        elgamal::ciphertext_add_assign(&mut private_balance, &coin.private_value);

        coin_store.private_balance = elgamal::compress_ciphertext(&private_balance);

        drop_veiled_coin(coin);
    }

    /// Withdraws the specifed private `amount` of veiled coin `CoinType` from the signing account.
    /// `range_proof_transferred_amount` is necessary for private VeiledCoin transactions as the
    /// amount sent will be private. If unwrapping some amount of VeiledCoin to Coin, the amount
    /// is necessarily public and can be checked outside of a range proof, so that `range_proof_transferred_amount` should be None. 
    public fun withdraw<CoinType>(
        account: &signer,
        withdraw_amount: Ciphertext,
        coin_store: &mut VeiledCoinStore<CoinType>,
        range_proof_updated_balance: &RangeProof,
        range_proof_transferred_amount: &Option<RangeProof>,
    ) {
        let account_addr = signer::address_of(account);
        assert!(
            has_veiled_coin_store<CoinType>(account_addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        let pubkey = &coin_store.pubkey;

        event::emit_event<WithdrawEvent>(
            &mut coin_store.withdraw_events,
            WithdrawEvent { },
        );

        // Update the coin store by homomorphically subtracting the committed withdrawn amount from the committed balance.
        let private_balance = elgamal::decompress_ciphertext(&coin_store.private_balance);

        elgamal::ciphertext_sub_assign(&mut private_balance, &withdraw_amount);

        // This function is splitting a commitment 'bal' into a commitment 'amount' and a commitment 'new_bal' =
        // = 'bal' - 'amount'. We assume that 'bal' is in [0, 2^{32}). To prevent underflow, we need to
        // verify a range proof that 'new_bal' is in [0, 2^{32}). In addition we need to verify a range proof 
    // that the transferred amount 'amount' is in [0, 2^{32}). Otherwise, a sender could send 'amount' = p-1
    // where p is the order of the scalar field, giving an updated balance of 
    // 'bal' - (p-1) mod p = 'bal' + 1. These checks ensure that 'bal' - 'amount' >= 0 
    // and therefore that 'bal' >= 'amount'. Note that when unwrapping some amount of 
    // VeiledCoin, `amount` will be public and already enforced to be 32 bits so that checking
        // the range proof for its size is unnecessary. 
        assert!(bulletproofs::verify_range_proof_elgamal(&private_balance, range_proof_updated_balance, pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST), ERANGE_PROOF_VERIFICATION_FAILED);
        if (std::option::is_some(range_proof_transferred_amount)) {
            assert!(bulletproofs::verify_range_proof_elgamal(&withdraw_amount, &std::option::extract(&mut *range_proof_transferred_amount), pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST), ERANGE_PROOF_VERIFICATION_FAILED);
        };
        coin_store.private_balance = elgamal::compress_ciphertext(&private_balance);
    }

    //
    // Tests
    //
    const SOME_RANDOMNESS_1: vector<u8> = x"a7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const SOME_RANDOMNESS_2: vector<u8> = x"b7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const SOME_RANDOMNESS_3: vector<u8> = x"c7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const SOME_RANDOMNESS_4: vector<u8> = x"d7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";

    #[testonly]
    fun generate_elgamal_keypair(seed: vector<u8>): (ristretto255::Scalar, elgamal::Pubkey) {
        // Hash the ristretto255 basepoint to get an arbitrary scalar for testing
        let priv_key = ristretto255::new_scalar_from_sha2_512(seed);
        let pubkey = elgamal::get_pubkey_from_scalar(&priv_key);
        (priv_key, pubkey)
    }

    const SOME_RANDOMNESS_5: vector<u8> = x"a2c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const SOME_RANDOMNESS_6: vector<u8> = x"b2c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const SOME_RANDOMNESS_7: vector<u8> = x"c2c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";
    const SOME_RANDOMNESS_8: vector<u8> = x"d2c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";

    #[test_only]                           
    // TODO: Describe in comment
    public fun generate_sigma_proof<CoinType>(
        source_pubkey: &elgamal::Pubkey, 
        dest_pubkey: &elgamal::Pubkey, 
        source_balance_ct: &elgamal::Ciphertext, 
        withdraw_ct: &elgamal::Ciphertext, 
        deposit_ct: &elgamal::Ciphertext,
        r: &ristretto255::Scalar, 
        sk: &ristretto255::Scalar, 
        transferred_amount: &ristretto255::Scalar,
        updated_source_balance: &ristretto255::Scalar): SigmaProof<CoinType> 
   {
        let x1 = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS_1);
        let x2 = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS_2);
        let x3 = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS_3);
        let x4 = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS_4);

        // X1 <- g^{x1}
        let big_x1 = ristretto255::basepoint_mul(&x1);

        // X2 <- g^{x2}
        let big_x2 = ristretto255::basepoint_mul(&x2);

        // X3 <- g^{x3}y^{x1}
        let big_x3 = ristretto255::basepoint_mul(&x3);
        let source_pubkey_point = elgamal::get_point_from_pubkey(source_pubkey);
        let source_pk_x1 = ristretto255::point_mul(&source_pubkey_point, &x1);
        ristretto255::point_add_assign(&mut big_x3, &source_pk_x1);
        
        // X4 <- g^{x3}\bar{y}^{x1}
        let big_x4 = ristretto255::basepoint_mul(&x3);
        let dest_pubkey_point = elgamal::get_point_from_pubkey(dest_pubkey);
        let dest_pk_x1 = ristretto255::point_mul(&dest_pubkey_point, &x1);
        ristretto255::point_add_assign(&mut big_x4, &dest_pk_x1);

        // X5 <- g^{x4}(C_R/D)^{x2}
        let big_x5 = ristretto255::basepoint_mul(&x4);
        let (c_L, c_R) = elgamal::ciphertext_as_points(source_balance_ct);
        let (big_c, big_d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let neg_d = ristretto255::point_neg(big_d);
        let c_R_acc = ristretto255::point_add(c_R, &neg_d);
        ristretto255::point_mul_assign(&mut c_R_acc, &x2);
        ristretto255::point_add_assign(&mut big_x5, &c_R_acc);

        // c <- H(g,y,\bar{y},C_L,C_R,C,D,\bar{C},X_1,X_2,X_3,X_4,X_5)
        let hash_input = vector::empty<u8>();

        let basepoint_bytes = ristretto255::point_to_bytes(&ristretto255::basepoint_compressed());
        vector::append<u8>(&mut hash_input, basepoint_bytes);

        let y = elgamal::get_compressed_point_from_pubkey(source_pubkey);
        let y_bytes = ristretto255::point_to_bytes(&y);
        vector::append<u8>(&mut hash_input, y_bytes);

        let y_bar = elgamal::get_compressed_point_from_pubkey(dest_pubkey);
        let y_bar_bytes = ristretto255::point_to_bytes(&y_bar);
        vector::append<u8>(&mut hash_input, y_bar_bytes);

        let c_L_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c_L));
        vector::append<u8>(&mut hash_input, c_L_bytes);

        let c_R_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c_R));
        vector::append<u8>(&mut hash_input, c_R_bytes);

        let big_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(big_c));
        vector::append<u8>(&mut hash_input, big_c_bytes);

        let big_d_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(big_d));
        vector::append<u8>(&mut hash_input, big_d_bytes);

        let bar_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(bar_c));
        vector::append<u8>(&mut hash_input, bar_c_bytes);

        let x_1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&big_x1));
        vector::append<u8>(&mut hash_input, x_1_bytes);

        let x_2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&big_x2));
        vector::append<u8>(&mut hash_input, x_2_bytes);

        let x_3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&big_x3));
        vector::append<u8>(&mut hash_input, x_3_bytes);

        let x_4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&big_x4));
        vector::append<u8>(&mut hash_input, x_4_bytes);

        let x_5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&big_x5));
        vector::append<u8>(&mut hash_input, x_5_bytes);

        vector::append<u8>(&mut hash_input, FIAT_SHAMIR_SIGMA_DST);

        let c = ristretto255::new_scalar_from_sha2_512(hash_input);

        // alpha_1 <- x1 + c * r
        let alpha1 = ristretto255::scalar_mul(&c, r);
        ristretto255::scalar_add_assign(&mut alpha1, &x1);

        // alpha_2 <- x2 + c * sk
        let alpha2 = ristretto255::scalar_mul(&c, sk);
        ristretto255::scalar_add_assign(&mut alpha2, &x2);

        // alpha_3 <- x3 + c * b^*
        let alpha3 = ristretto255::scalar_mul(&c, transferred_amount);
        ristretto255::scalar_add_assign(&mut alpha3, &x3);

        // alpha_4 <- x4 + c * b'
        let alpha4 = ristretto255::scalar_mul(&c, updated_source_balance); 
        ristretto255::scalar_add_assign(&mut alpha4, &x4);

        SigmaProof {
            x1: big_x1,
            x2: big_x2,
            x3: big_x3,
            x4: big_x4,
            x5: big_x5,
            alpha1,
            alpha2,
            alpha3,
            alpha4
        }
    }

    // TODO: Describe in comment 
    #[test_only]
    public fun serialize_sigma_proof<CoinType>(proof: &SigmaProof<CoinType>): vector<u8> {
        let x1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x1));
        let x2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x2));
        let x3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x3));
        let x4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x4));
        let x5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x5));
        let alpha1_bytes = ristretto255::scalar_to_bytes(&proof.alpha1);
        let alpha2_bytes = ristretto255::scalar_to_bytes(&proof.alpha2);
        let alpha3_bytes = ristretto255::scalar_to_bytes(&proof.alpha3);
        let alpha4_bytes = ristretto255::scalar_to_bytes(&proof.alpha4); 

        let bytes = vector::empty<u8>();
        vector::append<u8>(&mut bytes, x1_bytes);
        vector::append<u8>(&mut bytes, x2_bytes);
        vector::append<u8>(&mut bytes, x3_bytes);
        vector::append<u8>(&mut bytes, x4_bytes);
        vector::append<u8>(&mut bytes, x5_bytes);
        vector::append<u8>(&mut bytes, alpha1_bytes);
        vector::append<u8>(&mut bytes, alpha2_bytes);
        vector::append<u8>(&mut bytes, alpha3_bytes);
        vector::append<u8>(&mut bytes, alpha4_bytes);
        bytes
    }

    #[test_only]
    /// Returns true if the balance at address `owner` equals `value`.
    /// Requires the ElGamal encryption randomness and public key as auxiliary inputs.
    public fun verify_opened_balance<CoinType>(owner: address, value: u64, randomness: &Scalar, pubkey: &Pubkey): bool acquires VeiledCoinStore {
        // compute the expected committed balance
        let value = new_scalar_from_u64(value);
        let expected_ct = elgamal::new_ciphertext_with_basepoint(&value, randomness, pubkey);

        // get the actual committed balance
        let actual_ct = elgamal::decompress_ciphertext(&private_balance<CoinType>(owner));

        elgamal::ciphertext_equals(&actual_ct, &expected_ct)
    }

    #[test]
    fun sigma_proof_verify_test() 
    {
       let (source_priv_key, source_pubkey) = generate_elgamal_keypair(SOME_RANDOMNESS_1); 
       let balance_rand = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS_2);
       let balance_val = new_scalar_from_u64(150);
       let transfer_val = new_scalar_from_u64(50);
       let (_, dest_pubkey) = generate_elgamal_keypair(SOME_RANDOMNESS_3);
       let balance_ct = elgamal::new_ciphertext_with_basepoint(&balance_val, &balance_rand, &source_pubkey);
       //let source_randomness = ristretto255::scalar_neg(&rand);
       let transfer_rand = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS_4);
       let (_, withdraw_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &source_pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
       let new_balance_val = new_scalar_from_u64(100);
       let (_, deposit_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &dest_pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

       let sigma_proof = generate_sigma_proof<coin::FakeMoney>(&source_pubkey, &dest_pubkey, &balance_ct, &withdraw_ct, &deposit_ct, &transfer_rand, &source_priv_key, &transfer_val, &new_balance_val);


       verify_withdrawal_sigma_protocol(&source_pubkey, &dest_pubkey, &balance_ct, &withdraw_ct, &deposit_ct, &sigma_proof);
    }

    #[test]
    fun sigma_proof_serialize_test() 
    {
       let (source_priv_key, source_pubkey) = generate_elgamal_keypair(SOME_RANDOMNESS_1); 
       let rand = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS_2);
       let val = new_scalar_from_u64(50);
       let (_, dest_pubkey) = generate_elgamal_keypair(SOME_RANDOMNESS_3);
       let balance_ct = elgamal::new_ciphertext_with_basepoint(&val, &rand, &source_pubkey);
       let source_randomness = ristretto255::scalar_neg(&rand);
       let (_, withdraw_ct) = bulletproofs::prove_range_elgamal(&val, &source_randomness, &source_pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
       let source_new_val = new_scalar_from_u64(100);
       let (_, deposit_ct) = bulletproofs::prove_range_elgamal(&val, &rand, &dest_pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

       let sigma_proof = generate_sigma_proof<coin::FakeMoney>(&source_pubkey, &dest_pubkey, &balance_ct, &withdraw_ct, &deposit_ct, &source_randomness, &source_priv_key, &val, &source_new_val);

       let sigma_proof_bytes = serialize_sigma_proof<coin::FakeMoney>(&sigma_proof);

       let deserialized_proof = std::option::extract<SigmaProof<coin::FakeMoney>>(&mut deserialize_sigma_proof<coin::FakeMoney>(sigma_proof_bytes));
       
       assert!(ristretto255::point_equals(&sigma_proof.x1, &deserialized_proof.x1), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x2, &deserialized_proof.x2), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x3, &deserialized_proof.x3), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x4, &deserialized_proof.x4), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x5, &deserialized_proof.x5), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha1, &deserialized_proof.alpha1), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha2, &deserialized_proof.alpha2), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha3, &deserialized_proof.alpha3), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha4, &deserialized_proof.alpha4), 1);
    }

    #[test(myself = @veiled_coin, source_fx = @aptos_framework, destination = @0x1337)]
    fun basic_viability_test(
        myself: signer,
        source_fx: signer,
        destination: signer
    ) acquires VeiledCoinMinter, VeiledCoinStore {
        // Initialize the `veiled_coin` module
        init_module(&myself);

        features::change_feature_flags(&source_fx, vector[features::get_bulletproofs_feature()], vector[]);

        // Set up two accounts so we can register a new coin type on them
        let source_addr = signer::address_of(&source_fx);
        account::create_account_for_test(source_addr);
        let destination_addr = signer::address_of(&destination);
        account::create_account_for_test(destination_addr);

        // Create some 1,000 fake money inside 'source'
        coin::create_fake_money(&source_fx, &destination, 1000);

        // Split 500 and 500 between source and destination
        coin::transfer<coin::FakeMoney>(&source_fx, destination_addr, 500);

        // Mint 100 veiled coins at source (requires registering a veiled coin store at 'source')
        let (source_priv_key, source_pubkey) = generate_elgamal_keypair(SOME_RANDOMNESS_1);
        let source_pubkey_bytes = elgamal::pubkey_to_bytes(&source_pubkey);
        register<coin::FakeMoney>(&source_fx, source_pubkey_bytes);
        wrap<coin::FakeMoney>(&source_fx, 150);

        // Transfer 50 of these veiled coins to destination
        let val = new_scalar_from_u64(50);
        let rand = ristretto255::new_scalar_from_sha2_512(SOME_RANDOMNESS_2);

        //let ct = elgamal::new_ciphertext_with_basepoint(&val, &rand, &source_pubkey);

        // This will be the balance left at the source, that we need to do a range proof for
        let source_randomness = ristretto255::scalar_neg(&rand);
        let source_new_val = new_scalar_from_u64(100);
        let (new_balance_range_proof, _) = bulletproofs::prove_range_elgamal(&source_new_val, &source_randomness, &source_pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        let (transferred_amount_range_proof, withdraw_ct) = bulletproofs::prove_range_elgamal(&val, &source_randomness, &source_pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        // Execute the veiled transaction: no one will be able to tell 50 coins are being transferred.
        let (_, dest_pubkey) = generate_elgamal_keypair(SOME_RANDOMNESS_3);
        let dest_pubkey_bytes = elgamal::pubkey_to_bytes(&dest_pubkey);
        register<coin::FakeMoney>(&destination, dest_pubkey_bytes);

        let (_, deposit_ct) = bulletproofs::prove_range_elgamal(&source_new_val, &source_randomness, &dest_pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        let source_original_balance =  borrow_global<VeiledCoinStore<coin::FakeMoney>>(source_addr).private_balance;
        let sigma_proof = generate_sigma_proof<coin::FakeMoney>(&source_pubkey, &dest_pubkey, &elgamal::decompress_ciphertext(&source_original_balance), &withdraw_ct, &deposit_ct, &source_randomness, &source_priv_key, &val, &source_new_val);
        let sigma_proof_bytes = serialize_sigma_proof<coin::FakeMoney>(&sigma_proof);
        private_transfer_to_entry<coin::FakeMoney>(&source_fx, destination_addr, elgamal::ciphertext_to_bytes(&withdraw_ct), elgamal::ciphertext_to_bytes(&deposit_ct), bulletproofs::range_proof_to_bytes(&new_balance_range_proof), bulletproofs::range_proof_to_bytes(&transferred_amount_range_proof), sigma_proof_bytes);

        // Sanity check veiled balances
        assert!(verify_opened_balance<coin::FakeMoney>(source_addr, 50, &source_randomness, &source_pubkey), 1);
        assert!(verify_opened_balance<coin::FakeMoney>(destination_addr, 50, &rand, &dest_pubkey), 1);

        // TODO: Fix this
        /*assert!(point_equals(
            pedersen::commitment_as_point(&comm),
            &point_decompress(&private_balance<coin::FakeMoney>(destination_addr))
        ), 1);

        assert!(point_equals(
            pedersen::commitment_as_point(&source_new_comm),
            &point_decompress(&private_balance<coin::FakeMoney>(source_addr))
        ), 1);*/
    }
}
