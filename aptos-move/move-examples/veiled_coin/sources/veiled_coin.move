/// This module provides a veiled coin type, denoted `VeiledCoin<T>` that hides the value/denomination of a coin.
/// Importantly, although veiled transactions hide the amount of coins sent they still leak the sender and recipient.
///
/// ## Terminology
///
/// 1. *Veiled coin*: a coin whose value is secret; i.e., it is encrypted under the owner's public key
///
/// 2. *Veiled amount*: any amount that is secret; i.e., encrypted under some public key
///
/// 3. *Veiled transaction*: a transaction that hides its amount transferred; i.e., a transaction whose amount is veiled
///
/// 4. *Veiled balance*: unlike a normal balance, a veiled balance is secret; i.e., it is encrypted under the account's
///    public key
///
/// ## Limitations
///
/// **WARNING:** This module is **experimental**! It is *NOT* production-ready. Specifically:
///
///  1. deploying this module will likely lead to lost funds
///  2. this module has not been cryptographically-audited
///  3. the current implementation is vulnerable to _front-running attacks_ as described in the Zether paper [BAZB20].
///  4. there is no integration with wallet software which, for veiled accounts, must maintain an additional ElGamal
///    encryption keypair
///  5. there is no support for rotating the ElGamal encryption public key of a veiled account
///
/// ### Veiled coin amounts as truncated `u32`'s
///
/// Veiled coin amounts must be specified as `u32`'s rather than `u64`'s as would be typical for normal coins in the
/// Aptos framework. This is because coin amounts must be encrypted with an *efficient*, additively-homomorphic encryption
/// scheme. Currently, our best candidate is ElGamal encryption in the exponent, which can only decrypt values around
/// 32 bits or slightly larger.
///
/// Specifically, veiled coins are the middle 32 bits of the normal 64 bit coin values. In order to convert a `u32`
/// veiled coin amount to a normal `u64` coin amount, we have to shift it left by 16 bits.
///
/// ```
///   u64 normal coin amount format:
///   [ left    || middle  || right ]
///   [ 63 - 32 || 31 - 16 || 15 - 0]
///
///   u32 veiled coin amount format; we take the middle 32 bits from the `u64` format above and store them in a `u32`:
///   [ middle ]
///   [ 31 - 0 ]
/// ```
///
/// Recall that: A coin has a *decimal precision* $d$ (e.g., for `AptosCoin`, $d = 8$; see `initialize` in
/// `aptos_coin.move`). This precision $d$ is used when displaying a `u64` amount, by dividing the amount by $10^d$.
/// For example, if the precision $d = 2$, then a `u64` amount of 505 coins displays as 5.05 coins.
///
/// For veield coins, we can easily display a `u32` `Coin<T>` amount $v$ by:
///  1. Casting $v$ as a u64 and shifting this left by 16 bits, obtaining a 64-bit $v'$
///  2. Displaying $v'$ normally, by dividing it by $d$, which is the precision in `CoinInfo<T>`.
///
/// ## Implementation details
///
/// This module leverages a so-called "resource account," which helps us mint a `VeiledCoin<T>` from a
/// normal `coin::Coin<T>` by transferring this latter coin into a `coin::CoinStore<T>` stored in the
/// resource account.
///
/// Later on, when someone wants to convert their `VeiledCoin<T>` into a normal `coin::Coin<T>`,
/// the resource account can be used to transfer out the normal from its coin store. Transfering out a coin like this
/// requires a `signer` for the resource account, which the `veiled_coin` module can obtain via a `SignerCapability`.
///
/// ## TODOs
///
///  - We could have an `is_veiled` flag associated with the veiled balance, which we turn on only after a veiled to
///    veiled transaction to that account. This way, the wallet could even display the (actually-)veiled amount correctly.
///
/// ## References
///
/// [BAZB20] Zether: Towards Privacy in a Smart Contract World; by Bunz, Benedikt and Agrawal, Shashank and Zamani,
/// Mahdi and Boneh, Dan; in Financial Cryptography and Data Security; 2020
module veiled_coin::veiled_coin {
    use std::error;
    use std::signer;
    use std::vector;
    use std::option::Option;

    use aptos_std::elgamal::Self;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, scalar_zero};

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::bulletproofs::RangeProof;
    use aptos_std::bulletproofs;
    use aptos_std::pedersen;

    #[test_only]
    use std::features;
    #[test_only]
    use std::string::utf8;
    #[test_only]
    use aptos_std::debug::print;

    //
    // Errors
    //

    /// The range proof system does not support proofs for any number \in [0, 2^{32})
    const ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE : u64 = 1;

    /// A range proof failed to verify.
    const ERANGE_PROOF_VERIFICATION_FAILED : u64 = 2;

    /// Account already has `VeiledCoinStore<CoinType>` registered.
    const EVEILED_COIN_STORE_ALREADY_PUBLISHED: u64 = 3;

    /// Account hasn't registered `VeiledCoinStore<CoinType>`.
    const EVEILED_COIN_STORE_NOT_PUBLISHED: u64 = 4;

    /// Not enough coins to complete transaction.
    const EINSUFFICIENT_BALANCE: u64 = 5;

    /// Failed deserializing bytes into either ElGamal ciphertext or $\Sigma$-protocol proof.
    const EDESERIALIZATION_FAILED: u64 = 6;

    /// Byte vector given for deserialization was the wrong length.
    const EBYTES_WRONG_LENGTH: u64 = 7;

    /// $\Sigma$-protocol proof for withdrawals did not verify.
    const ESIGMA_PROTOCOL_VERIFY_FAILED: u64 = 8;

    /// Tried cutting out more elements than are in the vector via `cut_vector`.
    const EVECTOR_CUT_TOO_LARGE: u64 = 9;

    /// The `NUM_LEAST_SIGNIFICANT_BITS_REMOVED` and `NUM_MOST_SIGNIFICANT_BITS_REMOVED` constants need to sum to 32 (bits).
    const EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT: u64 = 10;

    /// Non-specific internal error (see source code)
    const EINTERNAL_ERROR: u64 = 11;

    //
    // Constants
    //

    /// The maximum number of bits used to represent a coin's value.
    const MAX_BITS_IN_VALUE : u64 = 32;

    /// When converting a `u64` normal (public) amount to a `u32` veiled amount, we keep the middle 32 bits and
    /// remove the `NUM_LEAST_SIGNIFICANT_BITS_REMOVED` least significant bits and the `NUM_MOST_SIGNIFICANT_BITS_REMOVED`
    /// most significant bits (see comments in the beginning of this file).
    ///
    /// When converting a `u32` veiled amount to a `u64` normal (public) amount, we simply cast it to `u64` and shift it
    /// left by `NUM_LEAST_SIGNIFICANT_BITS_REMOVED`.
    const NUM_LEAST_SIGNIFICANT_BITS_REMOVED: u8 = 16;

    /// See `NUM_LEAST_SIGNIFICANT_BITS_REMOVED` comments.
    const NUM_MOST_SIGNIFICANT_BITS_REMOVED: u8 = 16;

    /// The domain separation tag (DST) used for the Bulletproofs prover.
    const VEILED_COIN_DST : vector<u8> = b"AptosVeiledCoin/BulletproofRangeProof";

    /// The domain separation tag (DST) used in the Fiat-Shamir transform of our $\Sigma$-protocol.
    const FIAT_SHAMIR_SIGMA_DST : vector<u8> = b"AptosVeiledCoin/WithdrawalProofFiatShamir";

    //
    // Structs
    //

    /// Main structure representing a coin in an account's custody.
    struct VeiledCoin<phantom CoinType> {
        /// ElGamal ciphertext which encrypts the number of coins $v \in [0, 2^{32})$. This $[0, 2^{32})$ range invariant
        /// is enforced throughout the code via Bulletproof-based ZK range proofs.
        veiled_amount: elgamal::Ciphertext,
    }

    /// A holder of a specific coin type and its associated event handles.
    /// These are kept in a single resource to ensure locality of data.
    struct VeiledCoinStore<phantom CoinType> has key {
        /// A ElGamal ciphertext of a value $v \in [0, 2^{32})$, an invariant that is enforced throughout the code.
        veiled_balance: elgamal::CompressedCiphertext,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
        pk: elgamal::CompressedPubkey,
    }

    /// Holds an `account::SignerCapability` for the resource account created when initializing this module. This
    /// resource account houses a `coin::CoinStore<T>` for every type of coin `T` that is veiled.
    struct VeiledCoinMinter has store, key {
        signer_cap: account::SignerCapability,
    }

    /// A cryptographic proof that ensures correctness of a veiled-to-veiled coin transfer.
    struct VeiledTransferProof<phantom CoinType> has drop {
        new_balance_proof: RangeProof,
        veiled_amount_proof: RangeProof,
        sigma_proof: FullSigmaProof<CoinType>,
    }

    /// A cryptographic proof that ensures correctness of a veiled-to-*unveiled* coin transfer.
    struct UnveiledWithdrawalProof<phantom CoinType> has drop {
        sigma_proof: ElGamalToPedSigmaProof<CoinType>,
        new_balance_proof: RangeProof,
    }

    /// A $\Sigma$-protocol proof used as part of a `VeiledTransferProof`.
    /// This proof encompasses the $\Sigma$-protocol from `ElGamalToPedSigmaProof`.
    /// (A more detailed description can be found in `verify_withdrawal_sigma_protocol`.)
    struct FullSigmaProof<phantom CoinType> has drop {
        x1: RistrettoPoint,
        x2: RistrettoPoint,
        x3: RistrettoPoint,
        x4: RistrettoPoint,
        x5: RistrettoPoint,
        x6: RistrettoPoint,
        x7: RistrettoPoint,
        alpha1: Scalar,
        alpha2: Scalar,
        alpha3: Scalar,
        alpha4: Scalar,
    }

    /// A $\Sigma$-protocol proof used as part of a `UnveiledWithdrawalProof`.
    /// (A more detailed description can be found in TODO: implement.)
    struct ElGamalToPedSigmaProof<phantom CoinType> has drop {
        // TODO: implement
    }

    /// Event emitted when some amount of veiled coins were deposited into an account.
    struct DepositEvent has drop, store {
        // We cannot leak any information about how much has been deposited.
    }

    /// Event emitted when some amount of veiled coins were withdrawn from an account.
    struct WithdrawEvent has drop, store {
        // We cannot leak any information about how much has been withdrawn.
    }

    //
    // Module initialization, done only once when this module is first published on the blockchain
    //

    /// Initializes a so-called "resource" account which will maintain a `coin::CoinStore<T>` resource for all `Coin<T>`'s
    /// that have been converted into a `VeiledCoin<T>`.
    fun init_module(deployer: &signer) {
        assert!(
            bulletproofs::get_max_range_bits() >= MAX_BITS_IN_VALUE,
            error::internal(ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE)
        );

        assert!(
            NUM_LEAST_SIGNIFICANT_BITS_REMOVED + NUM_MOST_SIGNIFICANT_BITS_REMOVED == 32,
            error::internal(EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT)
        );

        // Create the resource account. This will allow this module to later obtain a `signer` for this account and
        // transfer `Coin<T>`'s into its `CoinStore<T>` before minting a `VeiledCoin<T>`.
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

    /// Initializes a veiled coin store for the specified `user` account with that user's ElGamal encryption public key.
    /// Importantly, the user's wallet must retain their corresponding secret key.
    public entry fun register<CoinType>(user: &signer, pk: vector<u8>) {
        let pk = elgamal::new_pubkey_from_bytes(pk);
        register_internal<CoinType>(user, std::option::extract(&mut pk));
    }

    /// Sends a *public* `amount` of normal coins from `sender` to the `recipient`'s veiled balance.
    ///
    /// **WARNING:** This function *leaks* the transferred `amount`, since it is given as a public input.
    public entry fun veil_to<CoinType>(
        sender: &signer, recipient: address, amount: u32) acquires VeiledCoinMinter, VeiledCoinStore
    {
        let c = coin::withdraw<CoinType>(sender, cast_u32_to_u64_amount(amount));

        let vc = veiled_mint_from_coin(c);

        veiled_deposit<CoinType>(recipient, vc)
    }

    /// Like `veil_to`, except `owner` is both the sender and the recipient.
    ///
    /// This function can be used by the `owner` to initialize his veiled balance to a *public* value.
    ///
    /// **WARNING:** The initialized balance is *leaked*, since its initialized `amount` is public here.
    public entry fun veil<CoinType>(owner: &signer, amount: u32) acquires VeiledCoinMinter, VeiledCoinStore {
        veil_to<CoinType>(owner, signer::address_of(owner), amount)
    }

    /// Takes a *public* `amount` of `VeiledCoin<CoinType>` coins from `sender`, unwraps them to a `coin::Coin<CoinType>`,
    /// and sends them to `recipient`. Maintains secrecy of `sender`'s new balance.
    ///
    /// Requires a range proof on the new balance of the sender, to ensure the sender has enough money to send.
    /// No range proof is necessary for the `amount`, which is given as a public `u32` value.
    ///
    /// **WARNING:** This *leaks* the transferred `amount`, since it is a public `u32` argument.
    public entry fun unveil_to<CoinType>(
        sender: &signer,
        recipient: address,
        amount: u32,
        range_proof_new_balance: vector<u8>) acquires VeiledCoinStore, VeiledCoinMinter
    {
        let range_proof_new_balance = bulletproofs::range_proof_from_bytes(range_proof_new_balance);

        let c = unveiled_withdraw<CoinType>(
            sender,
            amount,
            &range_proof_new_balance);

        coin::deposit<CoinType>(recipient, c);
    }

    /// Like `unveil_to`, except the `sender` is also the recipient.
    public entry fun unveil<CoinType>(
        sender: &signer,
        amount: u32,
        range_proof_new_balance: vector<u8>) acquires VeiledCoinStore, VeiledCoinMinter
    {
        unveil_to<CoinType>(sender, signer::address_of(sender), amount, range_proof_new_balance)
    }

    /// Sends a *veiled* `amount` from `sender` to `recipient`. After this call, the balance of the `sender`
    /// and `recipient` remains (or becomes) secret.
    ///
    /// The sent amount remains secret; It is encrypted both under the sender's PK (in `withdraw_ct`) and under the
    /// recipient's PK (in `deposit_ct`) using the *same* ElGamal randomness.
    ///
    /// Requires a `VeiledTransferProof`; i.e.:
    /// 1. A range proof on the new balance of the sender, to ensure the sender has enough money to send (in
    ///    `range_proof_new_balance`),
    /// 2. A range proof on the transferred amount in `withdraw_ct`, to ensure the sender won't create coins out of thin
    ///    air (in `range_proof_veiled_amount`),
    /// 3. A $\Sigma$-protocol to prove that 'veiled_withdraw_amount' encrypts the same veiled amount as
    ///    'veiled_deposit_amount' with the same randomness (in `sigma_proof_bytes`).
    public entry fun fully_veiled_transfer<CoinType>(
        sender: &signer,
        recipient: address,
        withdraw_ct: vector<u8>,
        deposit_ct: vector<u8>,
        updated_balance_comm: vector<u8>,
        transfer_value_comm: vector<u8>,
        range_proof_new_balance: vector<u8>,
        range_proof_veiled_amount: vector<u8>,
        sigma_proof_bytes: vector<u8>) acquires VeiledCoinStore
    {
        let veiled_withdraw_amount = elgamal::new_ciphertext_from_bytes(withdraw_ct);
        assert!(std::option::is_some(&veiled_withdraw_amount), error::invalid_argument(EDESERIALIZATION_FAILED));

        let veiled_deposit_amount = elgamal::new_ciphertext_from_bytes(deposit_ct);
        assert!(std::option::is_some(&veiled_deposit_amount), error::invalid_argument(EDESERIALIZATION_FAILED));

        let updated_balance_comm = pedersen::new_commitment_from_bytes(updated_balance_comm);
        assert!(std::option::is_some(&updated_balance_comm), error::invalid_argument(EDESERIALIZATION_FAILED));

        let transfer_value = pedersen::new_commitment_from_bytes(transfer_value_comm);
        assert!(std::option::is_some(&transfer_value), error::invalid_argument(EDESERIALIZATION_FAILED));

        // This $\Sigma$-protocol proofs proves that `veiled_withdraw_amount` encrypts the same value using the same
        // randomness as `veiled_deposit_amount` (i.e., the amount being transferred). These two ciphertexts are
        // required as we need to update both the sender's and the recipient's balances, which use different public keys
        // and so must be updated with ciphertexts encrypted under their respective public keys.
        let sigma_proof = deserialize_sigma_proof<CoinType>(sigma_proof_bytes);
        assert!(std::option::is_some(&sigma_proof), error::invalid_argument(EDESERIALIZATION_FAILED));

        // Requires a range proof on the new balance of the sender, to ensure the sender has enough money to send, in
        // addition to a  range proof on the transferred amount.
        let new_balance_proof = bulletproofs::range_proof_from_bytes(range_proof_new_balance);
        let veiled_amount_proof = bulletproofs::range_proof_from_bytes(range_proof_veiled_amount);

        let transfer_proof = VeiledTransferProof {
            new_balance_proof,
            veiled_amount_proof,
            sigma_proof: std::option::extract(&mut sigma_proof)
        };

        fully_veiled_transfer_internal<CoinType>(
            sender,
            recipient,
            std::option::extract(&mut veiled_withdraw_amount),
            std::option::extract(&mut veiled_deposit_amount),
            std::option::extract(&mut updated_balance_comm),
            std::option::extract(&mut transfer_value),
            &transfer_proof,
        )
    }

    //
    // Public functions.
    //

    /// Clamps a `u64` normal public amount to a `u32` to-be-veiled amount.
    ///
    /// WARNING: Precision is lost here (see "Veiled coin amounts as truncated `u32`'s" in the top-level comments)
    ///
    /// (Unclear if this function will be needed.)
    public fun clamp_u64_to_u32_amount(amount: u64): u32 {
        // Removes the `NUM_MOST_SIGNIFICANT_BITS_REMOVED` most significant bits.
        amount << NUM_MOST_SIGNIFICANT_BITS_REMOVED;
        amount >> NUM_MOST_SIGNIFICANT_BITS_REMOVED;

        // Removes the other `32 - NUM_MOST_SIGNIFICANT_BITS_REMOVED` least significant bits.
        amount = amount >> NUM_LEAST_SIGNIFICANT_BITS_REMOVED;

        // We are now left with a 32-bit value
        (amount as u32)
    }

    /// Casts a `u32` to-be-veiled amount to a `u64` normal public amount. No precision is lost here.
    public fun cast_u32_to_u64_amount(amount: u32): u64 {
        (amount as u64) << NUM_MOST_SIGNIFICANT_BITS_REMOVED
    }

    /// Returns `true` if `addr` is registered to receive veiled coins of `CoinType`.
    public fun has_veiled_coin_store<CoinType>(addr: address): bool {
        exists<VeiledCoinStore<CoinType>>(addr)
    }

    /// Returns the ElGamal encryption of the value of `coin`.
    public fun veiled_amount<CoinType>(coin: &VeiledCoin<CoinType>): &elgamal::Ciphertext {
        &coin.veiled_amount
    }

    /// Returns the ElGamal encryption of the veiled balance of `owner` for the provided `CoinType`.
    public fun veiled_balance<CoinType>(owner: address): elgamal::CompressedCiphertext acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(owner),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        borrow_global<VeiledCoinStore<CoinType>>(owner).veiled_balance
    }

    /// Given an address `addr`, returns the ElGamal encryption public key associated with that address
    public fun encryption_public_key<CoinType>(addr: address): elgamal::CompressedPubkey acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED)
        );

        borrow_global_mut<VeiledCoinStore<CoinType>>(addr).pk
    }

    /// Returns the total supply of veiled coins
    public fun total_veiled_coins<CoinType>(): u64 acquires VeiledCoinMinter {
        let rsrc_acc_addr = signer::address_of(&get_resource_account_signer());
        assert!(coin::is_account_registered<CoinType>(rsrc_acc_addr), EINTERNAL_ERROR);

        coin::balance<CoinType>(rsrc_acc_addr)
    }

    /// Like `register`, but the public key is parsed in an `elgamal::CompressedPubkey` struct.
    /// TODO: Do we want to require a PoK of the SK here?
    public fun register_internal<CoinType>(user: &signer, pk: elgamal::CompressedPubkey) {
        let account_addr = signer::address_of(user);
        assert!(
            !has_veiled_coin_store<CoinType>(account_addr),
            error::already_exists(EVEILED_COIN_STORE_ALREADY_PUBLISHED),
        );

        // Note: There is no way to find an ElGamal SK such that the `(0_G, 0_G)` ciphertext below decrypts to a non-zero
        // value. We'd need to have `(r * G, v * G + r * pk) = (0_G, 0_G)`, which implies `r = 0` for any choice of PK/SK.
        // Thus, we must have `v * G = 0_G`, which implies `v = 0`.

        let coin_store = VeiledCoinStore<CoinType> {
            veiled_balance: elgamal::ciphertext_from_compressed_points(
                ristretto255::point_identity_compressed(), ristretto255::point_identity_compressed()),
            deposit_events: account::new_event_handle<DepositEvent>(user),
            withdraw_events: account::new_event_handle<WithdrawEvent>(user),
            pk,
        };
        move_to(user, coin_store);
    }

    /// Mints a veiled coin from a normal coin, shelving the normal coin into the resource account's coin store.
    ///
    /// **WARNING:** Fundamentally, there is no way to hide the value of the coin being minted here.
    public fun veiled_mint_from_coin<CoinType>(c: Coin<CoinType>): VeiledCoin<CoinType> acquires VeiledCoinMinter {
        // If there is no CoinStore<CoinType> in the resource account, create one.
        let rsrc_acc_signer = get_resource_account_signer();
        let rsrc_acc_addr = signer::address_of(&rsrc_acc_signer);
        if (!coin::is_account_registered<CoinType>(rsrc_acc_addr)) {
            coin::register<CoinType>(&rsrc_acc_signer);
        };

        // Move the normal coin into the coin store, so we can mint a veiled coin.
        // (There is no other way to drop a normal coin, for safety reasons, so moving it into a coin store is
        //  the only option.)
        let value_u64 = coin::value(&c);
        let value_u32 = clamp_u64_to_u32_amount(value_u64);
        let value = ristretto255::new_scalar_from_u32(
            value_u32
        );

        // Paranoid check: assert that the u64 coin value had only its middle 32 bits set
        assert!(cast_u32_to_u64_amount(value_u32) == value_u64, error::internal(EINTERNAL_ERROR));

        coin::deposit(rsrc_acc_addr, c);

        VeiledCoin<CoinType> {
            veiled_amount: elgamal::new_ciphertext_no_randomness(&value)
        }
    }

    /// Removes a *public* `amount` of veiled coins from `sender` and returns them as a normal `coin::Coin`.
    ///
    /// Requires a ZK range proof on the new balance of the `sender`, to ensure the `sender` has enough money to send.
    /// Since the `amount` is public, no ZK range proof on it is required.
    ///
    /// **WARNING:** This function *leaks* the public `amount`.
    public fun unveiled_withdraw<CoinType>(
        sender: &signer,
        amount: u32,
        new_balance_proof: &RangeProof): Coin<CoinType> acquires VeiledCoinStore, VeiledCoinMinter
    {
        let addr = signer::address_of(sender);

        assert!(has_veiled_coin_store<CoinType>(addr), error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED));

        let scalar_amount = ristretto255::new_scalar_from_u32(amount);
        let veiled_amount = elgamal::new_ciphertext_no_randomness(&scalar_amount);

        let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(addr);

        // Since `veiled_amount` was created from a `u32` public `amount`, no ZK range proof is needed for it.
        veiled_withdraw(veiled_amount, coin_store, new_balance_proof, &std::option::none());

        // Note: If the above `withdraw` aborts, the whole TXN aborts, so there are no atomicity issues.
        coin::withdraw(&get_resource_account_signer(), cast_u32_to_u64_amount(amount))
    }


    /// Like `fully_veiled_transfer`, except the ciphertext and proofs have been deserialized into their respective structs.
    public fun fully_veiled_transfer_internal<CoinType>(
        sender: &signer,
        recipient_addr: address,
        veiled_withdraw_amount: elgamal::Ciphertext,
        veiled_deposit_amount: elgamal::Ciphertext,
        updated_balance_comm: pedersen::Commitment,
        veiled_amount_comm: pedersen::Commitment,
        transfer_proof: &VeiledTransferProof<CoinType>) acquires VeiledCoinStore
    {
        let sender_addr = signer::address_of(sender);

        let sender_pk = encryption_public_key<CoinType>(sender_addr);
        let recipient_pk = encryption_public_key<CoinType>(recipient_addr);

        // Note: The `get_pk_from_addr` call from above already asserts that `sender_addr` has a coin store.
        let sender_coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(sender_addr);
        let balance = elgamal::decompress_ciphertext(&sender_coin_store.veiled_balance);
        elgamal::ciphertext_sub_assign(&mut balance, &veiled_withdraw_amount);

        // Checks that `veiled_withdraw_amount` and `veiled_deposit_amount` encrypt the same amount of coins, under the
        // sender and recipient's PKs, respectively, by verifying the $\Sigma$-protocol proof in `transfer_proof`.
        sigma_protocol_verify(
            &sender_pk,
            &recipient_pk,
            &veiled_withdraw_amount,
            &veiled_deposit_amount,
            &balance,
            &updated_balance_comm,
            &veiled_amount_comm,
            &transfer_proof.sigma_proof);

        // TODO: Pass in updated balance here instead of coin store
        // Verifies the range proofs in `transfer_proof` and withdraws `veiled_withdraw_amount` from the `sender`'s account.
        veiled_withdraw<CoinType>(
            veiled_withdraw_amount,
            sender_coin_store,
            &transfer_proof.new_balance_proof,
            &std::option::some(transfer_proof.veiled_amount_proof));

        // Creates a new veiled coin for the recipient.
        let vc = VeiledCoin<CoinType> { veiled_amount: veiled_deposit_amount };

        // Deposits `veiled_deposit_amount` into the recipient's account
        // (Note, if this aborts, the whole transaction aborts, so we do not need to worry about atomicity.)
        veiled_deposit(recipient_addr, vc);
    }

    /// Deposits a veiled `coin` at address `to_addr`.
    public fun veiled_deposit<CoinType>(to_addr: address, coin: VeiledCoin<CoinType>) acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(to_addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(to_addr);

        // Fetch the veiled balance
        let veiled_balance = elgamal::decompress_ciphertext(&coin_store.veiled_balance);

        // Subtract the veiled amount from it, homomorphically
        elgamal::ciphertext_add_assign(&mut veiled_balance, &coin.veiled_amount);

        // Update the veiled balance
        coin_store.veiled_balance = elgamal::compress_ciphertext(&veiled_balance);

        // Make sure the veiled coin is dropped so it cannot be double spent
        drop_veiled_coin(coin);

        // Once successful, emit an event that a veiled deposit occurred.
        event::emit_event<DepositEvent>(
            &mut coin_store.deposit_events,
            DepositEvent {},
        );
    }

    /// Withdraws a `veiled_amount` of coins from the specified coin store. Let `balance` denote its current
    /// *veiled* balance.
    ///
    /// **WARNING:** This function assumes that `veiled_amount` is correctly encrypted under the sender's PK. This
    /// is the case when either (1) the amount was veiled correctly from a public value or (2) a $\Sigma$-protocol proof
    /// over `veiled_amount` verified successfully.
    ///
    /// Always requires a ZK range proof `new_balance_proof` on `balance - amount`. When the veiled amount was NOT
    /// created from a public value, additionally requires a ZK range proof `veiled_amount_proof` on `amount`.
    public fun veiled_withdraw<CoinType>(
        veiled_amount: elgamal::Ciphertext,
        coin_store: &mut VeiledCoinStore<CoinType>,
        new_balance_proof: &RangeProof,
        veiled_amount_proof: &Option<RangeProof>)
    {
        // Fetch the ElGamal public key of the veiled account
        let pk = &coin_store.pk;

        // Fetch the veiled balance of the veiled account
        let veiled_balance = elgamal::decompress_ciphertext(&coin_store.veiled_balance);

        // Update the account's veiled balance by homomorphically subtracting the veiled amount from the veiled balance.
        elgamal::ciphertext_sub_assign(&mut veiled_balance, &veiled_amount);

        // This function checks if it is possible to withdraw a veiled `amount` from a veiled `bal`, obtaining a new
        // veiled balance `new_bal = bal - amount`. It maintains an invariant that `new_bal \in [0, 2^{32})` as follows.
        //
        //  1. We assume (by the invariant) that `bal \in [0, 2^{32})`.
        //
        //  2. We verify a ZK range proof that `amount \in [0, 2^{32})`. Otherwise, a sender could set `amount = p-1`
        //     where `p` is the order of the scalar field, which would give `new_bal = bal - (p-1) mod p = bal + 1`.
        //     Therefore, a malicious spender could create coins out of thin air for themselves.
        //
        //  3. We verify a ZK range proof that `new_bal \in [0, 2^{32})`. Otherwise, a sender could set `amount = bal + 1`,
        //     which would satisfy condition (2) from above but would give `new_bal = bal - (bal + 1) = -1`. Therefore,
        //     a malicious spender could spend more coins than they have.
        //
        // Altogether, these checks ensure that `bal - amount >= 0` (as integers) and therefore that `bal >= amount`
        // (again, as integers).
        //
        // When the caller of this function created the `veiled_amount` from a public `u32` value, the
        // `veiled_amount_proof` range proof is no longer necessary since the caller guarantees that condition (2) from
        // above holds.

        // Checks range condition (3)
        assert!(
            bulletproofs::verify_range_proof_elgamal(
                &veiled_balance,
                new_balance_proof,
                pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST
            ),
            error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
        );

        // Checks range condition (2), if the veiled amount did not originate from a public amount
        if (std::option::is_some(veiled_amount_proof)) {
            assert!(
                bulletproofs::verify_range_proof_elgamal(
                    &veiled_amount,
                    std::option::borrow(veiled_amount_proof),
                    pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST
                ),
                error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
            );
        };

        // Update the veiled balance to reflect the veiled withdrawal
        coin_store.veiled_balance = elgamal::compress_ciphertext(&veiled_balance);

        // Once everything succeeds, emit an event to indicate a veiled withdrawal occurred
        event::emit_event<WithdrawEvent>(
            &mut coin_store.withdraw_events,
            WithdrawEvent { },
        );
    }

    //
    // Private functions.
    //

    /// Given a vector `vec`, removes the last `cut_len` elements of `vec` and returns them in order. (This function
    /// exists because we did not like the interface of `std::vector::trim`.)
    fun cut_vector<T>(vec: &mut vector<T>, cut_len: u64): vector<T> {
        let len = vector::length(vec);
        let res = vector::empty();
        assert!(len >= cut_len, error::out_of_range(EVECTOR_CUT_TOO_LARGE));
        while (cut_len > 0) {
            vector::push_back(&mut res, vector::pop_back(vec));
            cut_len = cut_len - 1;
        };
        vector::reverse(&mut res);
        res
    }

    /// Returns a signer for the resource account storing all the normal coins that have been veiled.
    fun get_resource_account_signer(): signer acquires VeiledCoinMinter {
        account::create_signer_with_capability(&borrow_global<VeiledCoinMinter>(@veiled_coin).signer_cap)
    }

    /// Used internally to drop veiled coins that were split or joined.
    fun drop_veiled_coin<CoinType>(c: VeiledCoin<CoinType>) {
        let VeiledCoin<CoinType> { veiled_amount: _ } = c;
    }

    /// Deserializes and returns a `SigmaProof` given its byte representation (see protocol description in
    /// `sigma_protocol_verify`)
    ///
    /// Elements at the end of the `SigmaProof` struct are expected to be at the start  of the byte vector, and
    /// serialized using the serialization formats in the `ristretto255` module.
    fun deserialize_sigma_proof<CoinType>(proof_bytes: vector<u8>): Option<FullSigmaProof<CoinType>> {
        if (vector::length<u8>(&proof_bytes) != 352) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };

        let x1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x1 = ristretto255::new_point_from_bytes(x1_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x1)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x1 = std::option::extract<RistrettoPoint>(&mut x1);

        let x2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x2 = ristretto255::new_point_from_bytes(x2_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x2)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x2 = std::option::extract<RistrettoPoint>(&mut x2);

        let x3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x3 = ristretto255::new_point_from_bytes(x3_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x3)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x3 = std::option::extract<RistrettoPoint>(&mut x3);

        let x4_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x4 = ristretto255::new_point_from_bytes(x4_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x4)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x4 = std::option::extract<RistrettoPoint>(&mut x4);

        let x5_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x5 = ristretto255::new_point_from_bytes(x5_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x5)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x5 = std::option::extract<RistrettoPoint>(&mut x5);

        let x6_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x6 = ristretto255::new_point_from_bytes(x6_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x6)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x6 = std::option::extract<RistrettoPoint>(&mut x6);

        let x7_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x7 = ristretto255::new_point_from_bytes(x7_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x7)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let x7 = std::option::extract<RistrettoPoint>(&mut x7);

        let alpha1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha1 = ristretto255::new_scalar_from_bytes(alpha1_bytes);
        if (!std::option::is_some(&alpha1)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let alpha1 = std::option::extract(&mut alpha1);

        let alpha2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha2 = ristretto255::new_scalar_from_bytes(alpha2_bytes);
        if (!std::option::is_some(&alpha2)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let alpha2 = std::option::extract(&mut alpha2);

        let alpha3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha3 = ristretto255::new_scalar_from_bytes(alpha3_bytes);
        if (!std::option::is_some(&alpha3)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let alpha3 = std::option::extract(&mut alpha3);

        let alpha4_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha4 = ristretto255::new_scalar_from_bytes(alpha4_bytes);
        if (!std::option::is_some(&alpha4)) {
            return std::option::none<FullSigmaProof<CoinType>>()
        };
        let alpha4 = std::option::extract(&mut alpha4);

        std::option::some(FullSigmaProof {
            x1, x2, x3, x4, x5, x6, x7, alpha1, alpha2, alpha3, alpha4
        })
    }

    // TODO: Update comment
    /// Verifies a $\Sigma$-protocol proof necessary to ensure correctness of a veiled transfer.
    /// Specifically, this proof proves that `withdraw_ct` and `deposit_ct` encrypt the same amount $v$ using the same
    /// randomness $r$, with `sender_pk` and `recipient_pk` respectively.
    ///
    /// # Cryptographic details
    ///
    /// The proof argues knowledge of a witness $w$ such that a specific relation $R(x; w)$ is satisfied, for a public
    /// statement $x$ known to the verifier (i.e., known to the validators). We describe this relation below.
    ///
    /// The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
    ///  - $v$, the amount being transferred
    ///  - $r$, ElGamal encryption randomness
    ///
    /// (Note that the $\Sigma$-protocol's zero-knowledge property ensures the witness is not revealed.)
    ///
    /// The public statement $x$ in this relation consists of:
    ///  - $G$, the basepoint of a given elliptic curve
    ///  - $Y$, the sender's PK
    ///  - $Y'$, the recipient's PK
    ///  - $(C, D)$, the ElGamal encryption of $v$ under the sender's PK
    ///  - $(C', D)$, the ElGamal encryption of $v$ under the recipient's PK
    ///
    ///
    /// The relation, at a high level, and created two ciphertexts $(C, D)$ and $(C', D)$
    /// encrypting $v$ under the sender's PK and recipient's PK, respectively.:
    ///
    /// ```
    /// R(
    ///     x = [ Y, Y', (C, C', D), G]
    ///     w = [ v, r ]
    /// ) = {
    ///     C = v * G + r * Y
    ///     C' = v * G + r * Y'
    ///     D = r * G
    /// }
    /// ```
    ///
    /// A relation similar to this is also described on page 14 of the Zether paper [BAZB20] (just replace  $G$ -> $g$,
    /// $C'$ -> $\bar{C}$, $Y$ -> $y$, $Y'$ -> $\bar{y}$, $v$ -> $b^*$).
    ///
    /// Note the equations $C_L - C = b' G + sk (C_R - D)$ and $Y = sk G$ in the Zether paper are enforced
    /// programmatically by this smart contract and so are not needed in our $\Sigma$-protocol.
    fun sigma_protocol_verify<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        sender_updated_balance_ct: &elgamal::Ciphertext,
        sender_updated_balance_comm: &pedersen::Commitment,
        transfer_value: &pedersen::Commitment,
        proof: &FullSigmaProof<CoinType>)
    {
        let sender_pk_point = elgamal::pubkey_to_point(sender_pk);
        let recipient_pk_point = elgamal::pubkey_to_point(recipient_pk);
        let (big_c, d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (big_bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let (c1, c2) = elgamal::ciphertext_as_points(sender_updated_balance_ct);
        let c = pedersen::commitment_as_point(sender_updated_balance_comm);
        let bar_c = pedersen::commitment_as_point(transfer_value);
        let h = pedersen::randomness_base_for_bulletproof();

        // TODO: Can be optimized so we don't re-serialize the proof for Fiat-Shamir
        let rho = sigma_protocol_fiat_shamir<CoinType>(
            sender_pk, recipient_pk,
            withdraw_ct, deposit_ct,
            sender_updated_balance_ct,
            sender_updated_balance_comm, transfer_value,
            &proof.x1, &proof.x2, &proof.x3, &proof.x4,
            &proof.x5, &proof.x6, &proof.x7);

        let g_alpha2 = ristretto255::basepoint_mul(&proof.alpha2);
        // \rho * D + X1 =? \alpha_2 * g
        let d_acc = ristretto255::point_mul(d, &rho);
        ristretto255::point_add_assign(&mut d_acc, &proof.x1);
        assert!(ristretto255::point_equals(&d_acc, &g_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha1 = ristretto255::basepoint_mul(&proof.alpha1);
        // \rho * C + X2 =? \alpha_1 * g + \alpha_2 * y
        let big_c_acc = ristretto255::point_mul(big_c, &rho);
        ristretto255::point_add_assign(&mut big_c_acc, &proof.x2);
        let y_alpha2 = ristretto255::point_mul(&sender_pk_point, &proof.alpha2);
        ristretto255::point_add_assign(&mut y_alpha2, &g_alpha1);
        assert!(ristretto255::point_equals(&big_c_acc, &y_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * \bar{C} + X3 =? \alpha_1 * g + \alpha_2 * \bar{y}
        let big_bar_c_acc = ristretto255::point_mul(big_bar_c, &rho);
        ristretto255::point_add_assign(&mut big_bar_c_acc, &proof.x3);
        let y_bar_alpha2 = ristretto255::point_mul(&recipient_pk_point, &proof.alpha2);
        ristretto255::point_add_assign(&mut y_bar_alpha2, &g_alpha1);
        assert!(ristretto255::point_equals(&big_bar_c_acc, &y_bar_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha3 = ristretto255::basepoint_mul(&proof.alpha3);
        // \rho * c_1 + X4 =? \alpha_3 * g + \alpha_4 * y
        let c1_acc = ristretto255::point_mul(c1, &rho);
        ristretto255::point_add_assign(&mut c1_acc, &proof.x4);
        let y_alpha4 = ristretto255::point_mul(&sender_pk_point, &proof.alpha4);
        ristretto255::point_add_assign(&mut y_alpha4, &g_alpha3);
        assert!(ristretto255::point_equals(&c1_acc, &y_alpha4), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha4 = ristretto255::basepoint_mul(&proof.alpha4);
        // \rho * c_2 + X5 =? \alpha_4 * g
        let c2_acc = ristretto255::point_mul(c2, &rho);
        ristretto255::point_add_assign(&mut c2_acc, &proof.x5);
        assert!(ristretto255::point_equals(&c2_acc, &g_alpha4), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * c + X6 =? \alpha_3 * g + \alpha_4 * h
        let c_acc = ristretto255::point_mul(c, &rho);
        ristretto255::point_add_assign(&mut c_acc, &proof.x6);
        let h_alpha4 = ristretto255::point_mul(&h, &proof.alpha4);
        ristretto255::point_add_assign(&mut h_alpha4, &g_alpha3);
        assert!(ristretto255::point_equals(&c_acc, &h_alpha4), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // \rho * \bar{c} + X7 =? \alpha_1 * g + \alpha_2 * h
        let bar_c_acc = ristretto255::point_mul(bar_c, &rho);
        ristretto255::point_add_assign(&mut bar_c_acc, &proof.x7);
        let h_alpha2 = ristretto255::point_mul(&h, &proof.alpha2);
        ristretto255::point_add_assign(&mut h_alpha2, &g_alpha1);
        assert!(ristretto255::point_equals(&bar_c_acc, &h_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));
        }

    /// TODO: explain the challenge derivation as a function of the parameters
    /// Computes the challenge value as `c = H(g, y, \bar{y}, h, C, D, \bar{C}, c_1, c_2, c, \bar{c}, {X_i}_{i=1}^7)`
    /// for the $\Sigma$-protocol from `verify_withdrawal_sigma_protocol` using the Fiat-Shamir transform. The notation
    /// used above is from the Zether [BAZB20] paper.
    fun sigma_protocol_fiat_shamir<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        sender_updated_balance: &elgamal::Ciphertext,
        balance: &pedersen::Commitment,
        transfer_value: &pedersen::Commitment,
        x1: &RistrettoPoint,
        x2: &RistrettoPoint,
        x3: &RistrettoPoint,
        x4: &RistrettoPoint,
        x5: &RistrettoPoint,
        x6: &RistrettoPoint,
        x7: &RistrettoPoint): Scalar
    {
        let (big_c, d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (big_bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let (c1, c2) = elgamal::ciphertext_as_points(sender_updated_balance); 
        let c = pedersen::commitment_as_point(balance);
        let bar_c = pedersen::commitment_as_point(transfer_value); 

        let hash_input = vector::empty<u8>();

        let basepoint_bytes = ristretto255::point_to_bytes(&ristretto255::basepoint_compressed());
        vector::append<u8>(&mut hash_input, basepoint_bytes);

        let h_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&pedersen::randomness_base_for_bulletproof()));
        vector::append<u8>(&mut hash_input, h_bytes);

        let y = elgamal::pubkey_to_compressed_point(sender_pk);
        let y_bytes = ristretto255::point_to_bytes(&y);
        vector::append<u8>(&mut hash_input, y_bytes);

        let y_bar = elgamal::pubkey_to_compressed_point(recipient_pk);
        let y_bar_bytes = ristretto255::point_to_bytes(&y_bar);
        vector::append<u8>(&mut hash_input, y_bar_bytes);

        let big_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(big_c));
        vector::append<u8>(&mut hash_input, big_c_bytes);

        let d_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(d));
        vector::append<u8>(&mut hash_input, d_bytes);

        let bar_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(bar_c));
        vector::append<u8>(&mut hash_input, bar_c_bytes);

        let c1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c1));
        vector::append<u8>(&mut hash_input, c1_bytes);

        let c2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c2));
        vector::append<u8>(&mut hash_input, c2_bytes);

        let c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c));
        vector::append<u8>(&mut hash_input, c_bytes);

        let big_bar_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(big_bar_c));
        vector::append<u8>(&mut hash_input, big_bar_c_bytes);

        let x_1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x1));
        vector::append<u8>(&mut hash_input, x_1_bytes);

        let x_2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x2));
        vector::append<u8>(&mut hash_input, x_2_bytes);

        let x_3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x3));
        vector::append<u8>(&mut hash_input, x_3_bytes);

        let x_4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x4));
        vector::append<u8>(&mut hash_input, x_4_bytes);

        let x_5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x5));
        vector::append<u8>(&mut hash_input, x_5_bytes);

        let x_6_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x6));
        vector::append<u8>(&mut hash_input, x_6_bytes);

        let x_7_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x7));
        vector::append<u8>(&mut hash_input, x_7_bytes);

        vector::append<u8>(&mut hash_input, FIAT_SHAMIR_SIGMA_DST);

        ristretto255::new_scalar_from_sha2_512(hash_input)
    }

    //
    // Test-only functions
    //

    #[test_only]
    /// Returns a random ElGamal keypair
    fun generate_elgamal_keypair(): (Scalar, elgamal::CompressedPubkey) {
        let sk = ristretto255::random_scalar();
        let pk = elgamal::pubkey_from_secret_key(&sk);
        (sk, pk)
    }

    #[test_only]
    /// Returns true if the balance at address `owner` equals `value`.
    /// Requires the ElGamal encryption randomness `r` and public key `pk` as auxiliary inputs.
    public fun verify_opened_balance<CoinType>(
        owner: address, value: u32, r: &Scalar, pk: &elgamal::CompressedPubkey): bool acquires VeiledCoinStore
    {
        // compute the expected encrypted balance
        let value = ristretto255::new_scalar_from_u32(value);
        let expected_ct = elgamal::new_ciphertext_with_basepoint(&value, r, pk);

        // get the actual encrypted balance
        let actual_ct = elgamal::decompress_ciphertext(&veiled_balance<CoinType>(owner));

        elgamal::ciphertext_equals(&actual_ct, &expected_ct)
    }

    #[test_only]
    /// Proves the $\Sigma$-protocol used for veiled coin transfers.
    /// See `sigma_protocol_verify` for a detailed description of the $\Sigma$-protocol
    public fun sigma_protocol_prove<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        sender_updated_balance: &elgamal::Ciphertext,
        balance: &pedersen::Commitment,
        transfer_value: &pedersen::Commitment,
        amount_rand: &Scalar,
        amount_val: &Scalar,
        updated_balance_rand: &Scalar,
        updated_balance_val: &Scalar): FullSigmaProof<CoinType>
   {
        let x1 = ristretto255::random_scalar();
        let x2 = ristretto255::random_scalar();
        let x3 = ristretto255::random_scalar();
        let x4 = ristretto255::random_scalar();
        let source_pk_point = elgamal::pubkey_to_point(sender_pk);
        let recipient_pk_point = elgamal::pubkey_to_point(recipient_pk);
        let h = pedersen::randomness_base_for_bulletproof();

        // X1 <- x2 * g
        let big_x1 = ristretto255::basepoint_mul(&x2);

        // X2 <- x1 * g + x2 * y
        let big_x2 = ristretto255::basepoint_mul(&x1);
        let source_pk_x2 = ristretto255::point_mul(&source_pk_point, &x2);
        ristretto255::point_add_assign(&mut big_x2, &source_pk_x2);

        // X3 <- x1 * g + x2 * \bar{y}
        let big_x3 = ristretto255::basepoint_mul(&x1);
        let recipient_pk_x2 = ristretto255::point_mul(&recipient_pk_point, &x2);
        ristretto255::point_add_assign(&mut big_x3, &recipient_pk_x2);

        // X4 <- x3 * g + x4 * y
        let big_x4 = ristretto255::basepoint_mul(&x3);
        let source_pk_x4 = ristretto255::point_mul(&source_pk_point, &x4);
        ristretto255::point_add_assign(&mut big_x4, &source_pk_x4);

        // X5 <- x4 * g
        let big_x5 = ristretto255::basepoint_mul(&x4);

        // X6 <- x3 * g + x4 * h
        let big_x6 = ristretto255::basepoint_mul(&x3);
        let h_x4 = ristretto255::point_mul(&h, &x4);
        ristretto255::point_add_assign(&mut big_x6, &h_x4);

        // X7 <- x1 * g + x2 * h
        let big_x7 = ristretto255::basepoint_mul(&x1);
        let h_x2 = ristretto255::point_mul(&h, &x2);
        ristretto255::point_add_assign(&mut big_x7, &h_x2);


        let rho = sigma_protocol_fiat_shamir<CoinType>(
            sender_pk, recipient_pk,
            withdraw_ct, deposit_ct,
            sender_updated_balance,
            balance, transfer_value,
            &big_x1, &big_x2, &big_x3, &big_x4,
            &big_x5, &big_x6, &big_x7);

        // alpha_1 <- x1 + rho * v
        let alpha1 = ristretto255::scalar_mul(&rho, amount_val);
        ristretto255::scalar_add_assign(&mut alpha1, &x1);

        // alpha_2 <- x2 + \rho * r
        let alpha2 = ristretto255::scalar_mul(&rho, amount_rand);
        ristretto255::scalar_add_assign(&mut alpha2, &x2);

        // alpha_3 <- x3 + \rho * b
        let alpha3 = ristretto255::scalar_mul(&rho, updated_balance_val);
        ristretto255::scalar_add_assign(&mut alpha3, &x3);

        // alpha_4 <- x4 + \rho * r_b
        let alpha4 = ristretto255::scalar_mul(&rho, updated_balance_rand);
        ristretto255::scalar_add_assign(&mut alpha4, &x4);

        FullSigmaProof {
            x1: big_x1,
            x2: big_x2,
            x3: big_x3,
            x4: big_x4,
            x5: big_x5,
            x6: big_x6,
            x7: big_x7,
            alpha1,
            alpha2,
            alpha3,
            alpha4,
        }
    }

    #[test_only]
    /// Given a $\Sigma$-protocol proof, serializes it into byte form.
    /// Elements at the end of the `SigmaProof` struct are placed into the vector first,
    /// using the serialization formats in the `ristretto255` module.
    public fun serialize_sigma_proof<CoinType>(proof: &FullSigmaProof<CoinType>): vector<u8> {
        let x1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x1));
        let x2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x2));
        let x3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x3));
        let x4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x4));
        let x5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x5));
        let x6_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x6));
        let x7_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x7));
        let alpha1_bytes = ristretto255::scalar_to_bytes(&proof.alpha1);
        let alpha2_bytes = ristretto255::scalar_to_bytes(&proof.alpha2);
        let alpha3_bytes = ristretto255::scalar_to_bytes(&proof.alpha3);
        let alpha4_bytes = ristretto255::scalar_to_bytes(&proof.alpha4);

        let bytes = vector::empty<u8>();
        vector::append<u8>(&mut bytes, alpha4_bytes);
        vector::append<u8>(&mut bytes, alpha3_bytes);
        vector::append<u8>(&mut bytes, alpha2_bytes);
        vector::append<u8>(&mut bytes, alpha1_bytes);
        vector::append<u8>(&mut bytes, x7_bytes);
        vector::append<u8>(&mut bytes, x6_bytes);
        vector::append<u8>(&mut bytes, x5_bytes);
        vector::append<u8>(&mut bytes, x4_bytes);
        vector::append<u8>(&mut bytes, x3_bytes);
        vector::append<u8>(&mut bytes, x2_bytes);
        vector::append<u8>(&mut bytes, x1_bytes);

        bytes
    }

    #[test_only]
    /// Initializes the `veiled_coin` module and sets up a `sender` account with `sender_amount` + `recipient_amount`
    /// of `FakeCoin`'s. Then, sends `recipient_amount` of coins from `sender` to `recipient`.
    ///
    /// Can be called with `sender` set to be equal to `recipient`.
    fun set_up_for_veiled_coin_test(
        veiled_coin: &signer,
        aptos_fx: signer,
        sender: &signer,
        recipient: &signer,
        sender_amount: u32,
        recipient_amount: u32,
    ) {
        // Assumption is that framework address is different than recipient and sender addresses
        assert!(signer::address_of(&aptos_fx) != signer::address_of(sender), 1);
        assert!(signer::address_of(&aptos_fx) != signer::address_of(recipient), 2);

        // Initialize the `veiled_coin` module & enable the feature
        init_module(veiled_coin);
        println(b"Initialized module.");
        features::change_feature_flags(&aptos_fx, vector[features::get_bulletproofs_feature()], vector[]);
        println(b"Enabled feature flags.");

        // Set up an account for the framework address
        account::create_account_for_test(signer::address_of(&aptos_fx)); // needed in `coin::create_fake_money`
        account::create_account_for_test(signer::address_of(sender)); // needed in `coin::transfer`
        if (signer::address_of(recipient) != signer::address_of(sender)) {
            account::create_account_for_test(signer::address_of(recipient)); // needed in `coin::transfer`
        };
        println(b"Created accounts for test.");

        // Create `amount` of `FakeCoin` coins at the Aptos 0x1 address (must do) and register a `FakeCoin` coin
        // store for the `sender`.
        coin::create_fake_money(
            &aptos_fx,
            sender,
            cast_u32_to_u64_amount(sender_amount + recipient_amount));
        println(b"Created fake money inside @aptos_framework");

        // Transfer some coins from the framework to the sender
        coin::transfer<coin::FakeMoney>(
            &aptos_fx,
            signer::address_of(sender),
            cast_u32_to_u64_amount(sender_amount));
        println(b"Transferred some fake money to the sender.");

        // Transfer some coins from the sender to the recipient
        coin::register<coin::FakeMoney>(recipient);
        coin::transfer<coin::FakeMoney>(
            &aptos_fx,
            signer::address_of(recipient),
            cast_u32_to_u64_amount(recipient_amount));
        println(b"Transferred some fake money to the recipient.");

        println(b"Sender balance (as u64):");
        print(&coin::balance<coin::FakeMoney>(signer::address_of(sender)));
        println(b"Sender balance (as u32):");
        print(&clamp_u64_to_u32_amount(coin::balance<coin::FakeMoney>(signer::address_of(sender))));
        if (signer::address_of(recipient) != signer::address_of(sender)) {
            println(b"Recipient balance (as u64):");
            print(&coin::balance<coin::FakeMoney>(signer::address_of(recipient)));
            println(b"Sender balance (as u32):");
            print(&clamp_u64_to_u32_amount(coin::balance<coin::FakeMoney>(signer::address_of(recipient))));
        } else {
            println(b"(Recipient equals sender)");
        };
    }

    #[test_only]
    /// Prints a string on its own line.
    fun println(str: vector<u8>) {
        print(&utf8(str));
    }

    //
    // Tests
    //

    #[test]
    fun sigma_proof_verify_test()
    {
        // Pick a keypair for the sender, and one for the recipient
        let (_, sender_pk) = generate_elgamal_keypair();
        let (_, recipient_pk) = generate_elgamal_keypair();

        // Set the transferred amount to 50
        let amount_val = ristretto255::new_scalar_from_u32(50);
        let amount_rand = ristretto255::random_scalar();
        // Encrypt the amount under the sender's PK
        let withdraw_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &sender_pk);

        // Encrypt the amount under the recipient's PK
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &recipient_pk);

        let value_comm = pedersen::new_commitment_for_bulletproof(&amount_val, &amount_rand);

        // Set sender's new balance after the transaction to 100
        let updated_balance_val = ristretto255::new_scalar_from_u32(100);
        let updated_balance_rand = ristretto255::random_scalar();
        let updated_balance_ct = elgamal::new_ciphertext_with_basepoint(&updated_balance_val, &updated_balance_rand, &sender_pk);

        let updated_balance_comm = pedersen::new_commitment_for_bulletproof(&updated_balance_val, &updated_balance_rand);

        let sigma_proof = sigma_protocol_prove<coin::FakeMoney>(
            &sender_pk,
            &recipient_pk,
            &withdraw_ct,           // withdrawn amount, encrypted under sender PK
            &deposit_ct,            // deposited amount, encrypted under recipient PK (same plaintext as `withdraw_ct`)
            &updated_balance_ct,    // sender's balance after the transaction goes through, encrypted under sender PK
            &updated_balance_comm,  // commitment to sender's balance to prevent range proof forgery
            &value_comm,            // commitment to transfer amount to prevent range proof forgery
            &amount_rand,           // encryption randomness for `withdraw_ct` and `deposit_ct`
            &amount_val,            // transferred amount
            &updated_balance_rand,  // encryption randomness for updated balance ciphertext
            &updated_balance_val,   // sender's balance after the transfer
        );

        sigma_protocol_verify(
            &sender_pk, 
            &recipient_pk, 
            &withdraw_ct, 
            &deposit_ct, 
            &updated_balance_ct, 
            &updated_balance_comm, 
            &value_comm, 
            &sigma_proof
        );
    }

    #[test]
    #[expected_failure(abort_code = 0x10008, location = Self)]
    fun sigma_proof_verify_fails_test()
    {
       let (_, source_pk) = generate_elgamal_keypair();
       let transfer_val = ristretto255::new_scalar_from_u32(50);
       let (_, dest_pk) = generate_elgamal_keypair();
       let transfer_rand = ristretto255::random_scalar();
       let (_, withdraw_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &source_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
       let (_, deposit_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &dest_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
       let value_comm = pedersen::new_commitment_for_bulletproof(&transfer_val, &transfer_rand);
       let updated_balance_val = ristretto255::new_scalar_from_u32(100);
       let updated_balance_rand = ristretto255::random_scalar();
       let updated_balance_ct = elgamal::new_ciphertext_with_basepoint(&updated_balance_val, &updated_balance_rand, &source_pk);

       let updated_balance_comm = pedersen::new_commitment_for_bulletproof(&updated_balance_val, &updated_balance_rand);

       let sigma_proof = sigma_protocol_prove<coin::FakeMoney>(&source_pk, &dest_pk, &withdraw_ct, &deposit_ct, &updated_balance_ct, &updated_balance_comm, &value_comm, &transfer_rand, &transfer_val, &updated_balance_rand, &updated_balance_val);

       let random_point = ristretto255::random_point();
       sigma_proof.x1 = random_point;

       sigma_protocol_verify(&source_pk, &dest_pk, &withdraw_ct, &deposit_ct, &updated_balance_ct, &updated_balance_comm, &value_comm, &sigma_proof);
    }

    #[test]
    fun sigma_proof_serialize_test()
    {
       let (_, source_pk) = generate_elgamal_keypair();
       let transfer_val = ristretto255::new_scalar_from_u32(50);
       let (_, dest_pk) = generate_elgamal_keypair();
       let transfer_rand = ristretto255::random_scalar();
       let (_, withdraw_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &source_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
       let (_, deposit_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &dest_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
       let value_comm = pedersen::new_commitment_for_bulletproof(&transfer_val, &transfer_rand);
       let updated_balance_val = ristretto255::new_scalar_from_u32(100);
       let updated_balance_rand = ristretto255::random_scalar();
       let updated_balance_ct = elgamal::new_ciphertext_with_basepoint(&updated_balance_val, &updated_balance_rand, &source_pk);
       let updated_balance_comm = pedersen::new_commitment_for_bulletproof(&updated_balance_val, &updated_balance_rand);

       let sigma_proof = sigma_protocol_prove<coin::FakeMoney>(&source_pk, &dest_pk, &withdraw_ct, &deposit_ct, &updated_balance_ct, &updated_balance_comm, &value_comm, &transfer_rand, &transfer_val, &updated_balance_rand, &updated_balance_val);




       let sigma_proof_bytes = serialize_sigma_proof<coin::FakeMoney>(&sigma_proof);

       let deserialized_proof = std::option::extract<FullSigmaProof<coin::FakeMoney>>(&mut deserialize_sigma_proof<coin::FakeMoney>(sigma_proof_bytes));

       assert!(ristretto255::point_equals(&sigma_proof.x1, &deserialized_proof.x1), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x2, &deserialized_proof.x2), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x3, &deserialized_proof.x3), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x4, &deserialized_proof.x4), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x5, &deserialized_proof.x5), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x6, &deserialized_proof.x6), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x7, &deserialized_proof.x7), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha1, &deserialized_proof.alpha1), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha2, &deserialized_proof.alpha2), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha3, &deserialized_proof.alpha3), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha4, &deserialized_proof.alpha4), 1);
    }

    #[test(veiled_coin = @veiled_coin, aptos_fx = @aptos_framework, sender = @0xc0ffee, recipient = @0x1337)]
    fun veil_test(
        veiled_coin: signer,
        aptos_fx: signer,
        sender: signer,
        recipient: signer
    ) acquires VeiledCoinMinter, VeiledCoinStore {
        println(b"Starting veil_test()...");
        print(&@veiled_coin);
        print(&@aptos_framework);
        // TODO: This line seems to yield a strange, irreproducible invariant violation error...
        //assert!(@veiled_coin != @aptos_framework, 1);

        // Split 500 and 500 between `sender` and `recipient`
        set_up_for_veiled_coin_test(
            &veiled_coin, aptos_fx, &sender, &recipient, 500u32, 500u32);

        // Register a veiled balance at the `recipient`'s account
        let (_, recipient_pk) = generate_elgamal_keypair();
        register<coin::FakeMoney>(&recipient, elgamal::pubkey_to_bytes(&recipient_pk));
        println(b"Registered recipient's veiled coin balance");

        // Veil 150 normal coins from the `sender`'s normal coin account to the `recipient`'s veiled coin account
        veil_to<coin::FakeMoney>(&sender, signer::address_of(&recipient), 150u32);
        println(b"Sender veiled some coins over to the recipient");

        // Check the transfer occurred correctly: sender has 350 public coins, recipient has 150 (not-yet-)veiled coins
        assert!(coin::balance<coin::FakeMoney>(signer::address_of(&sender)) == cast_u32_to_u64_amount(350u32), 1);
        assert!(verify_opened_balance<coin::FakeMoney>(
            signer::address_of(&recipient), 150u32, &ristretto255::scalar_zero(), &recipient_pk), 1);

        // Register a veiled balance at the `sender`'s account
        let (_, sender_pk) = generate_elgamal_keypair();
        register<coin::FakeMoney>(&sender, elgamal::pubkey_to_bytes(&sender_pk));

        // The `recipient` wants to unveil 50 coins (to the `sender`), so build a range proof for that.
        // (Note: Technically, because the balance is not yet actually-veiled, the range proof could be avoided here in
        //  a smarter design.)
        let recipient_new_balance = ristretto255::new_scalar_from_u32(100u32);
        let (new_balance_range_proof, _) = bulletproofs::prove_range_elgamal(
            &recipient_new_balance, &ristretto255::scalar_zero(),
            &recipient_pk,
            MAX_BITS_IN_VALUE, VEILED_COIN_DST);
        let new_balance_range_proof_bytes = bulletproofs::range_proof_to_bytes(&new_balance_range_proof);

        // Transfer `50` veiled coins from the `recipient` to the `sender`'s public balance
        unveil_to<coin::FakeMoney>(
            &recipient, signer::address_of(&sender), 50u32, new_balance_range_proof_bytes);

        // Check that the sender now has 350 + 50 = 400 public coins
        let sender_public_balance = coin::balance<coin::FakeMoney>(signer::address_of(&sender));
        assert!(sender_public_balance == cast_u32_to_u64_amount(400u32), 1);
        // Check that the recipient now has 100 veiled coins
        assert!(verify_opened_balance<coin::FakeMoney>(
            signer::address_of(&recipient), 100u32, &ristretto255::scalar_zero(), &recipient_pk), 1);
    }

    #[test(veiled_coin = @veiled_coin, aptos_fx = @aptos_framework, sender = @0x1337)]
    fun unveil_test(
        veiled_coin: signer,
        aptos_fx: signer,
        sender: signer,
    ) acquires VeiledCoinMinter, VeiledCoinStore {
        println(b"Starting unveil_test()...");
        print(&@veiled_coin);
        print(&@aptos_framework);
        // TODO: This line seems to yield a strange, irreproducible invariant violation error...
        //assert!(@veiled_coin != @aptos_framework, 1);
        // This line does not
        //assert!(signer::address_of(&veiled_coin) != signer::address_of(&aptos_fx), 1);

        // Create a `sender` account with 500 `FakeCoin`'s
        set_up_for_veiled_coin_test(
            &veiled_coin, aptos_fx, &sender, &sender, 500, 0);

        // Register a veiled balance for the `sender`
        let (_, sender_pk) = generate_elgamal_keypair();
        register<coin::FakeMoney>(&sender, elgamal::pubkey_to_bytes(&sender_pk));
        println(b"Registered the sender's veiled balance");

        // Veil 150 out of the `sender`'s 500 coins.
        //
        // Note: Sender initializes his veiled balance to 150 veiled coins, which is why we don't need its SK to decrypt
        // it in order to transact.
        veil<coin::FakeMoney>(&sender, 150);
        println(b"Veiled 150 coins to the `sender`");

        println(b"Total veiled coins:");
        print(&total_veiled_coins<coin::FakeMoney>());

        assert!(total_veiled_coins<coin::FakeMoney>() == cast_u32_to_u64_amount(150), 1);

        // The `unveil` function uses randomness zero for the ElGamal encryption of the amount
        let sender_new_balance = ristretto255::new_scalar_from_u32(100);
        let zero_randomness = ristretto255::scalar_zero();

        // TODO: Will need a different wrapper function that creates the `UnveiledWithdrawalProof`
        let (new_balance_range_proof, _) = bulletproofs::prove_range_elgamal(
            &sender_new_balance,
            &zero_randomness,
            &sender_pk,
            MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        // Move 50 veiled coins into the public balance of the sender
        unveil<coin::FakeMoney>(
            &sender,
            50,
            bulletproofs::range_proof_to_bytes(&new_balance_range_proof));

        println(b"Remaining veiled coins, after `unveil` call:");
        print(&total_veiled_coins<coin::FakeMoney>());

        assert!(total_veiled_coins<coin::FakeMoney>() == cast_u32_to_u64_amount(100), 1);

        assert!(verify_opened_balance<coin::FakeMoney>(
            signer::address_of(&sender), 100, &zero_randomness, &sender_pk), 2);

        let remaining_public_balance = coin::balance<coin::FakeMoney>(signer::address_of(&sender));
        assert!(remaining_public_balance == cast_u32_to_u64_amount(400), 3);
    }

    // TODO: test that payments to self return successfully (ideally, they should do nothing)

    #[test(veiled_coin = @veiled_coin, aptos_fx = @aptos_framework, sender = @0xc0ffee, recipient = @0x1337)]
    fun basic_viability_test(
        veiled_coin: signer,
        aptos_fx: signer,
        sender: signer,
        recipient: signer
    ) acquires VeiledCoinMinter, VeiledCoinStore {
        set_up_for_veiled_coin_test(&veiled_coin, aptos_fx, &sender, &recipient, 500, 500);

        // Creates a balance of `b = 150` veiled coins at sender (requires registering a veiled coin store at 'sender')
        let (_, sender_pk) = generate_elgamal_keypair();
        register<coin::FakeMoney>(&sender, elgamal::pubkey_to_bytes(&sender_pk));
        veil<coin::FakeMoney>(&sender, 150);
        println(b"Veiled 150 coins to the `sender`");
        // TODO: these throw an invariant violation
        //print(&sender);
        //print(&signer::address_of(&sender));

        // Make sure we are correctly keeping track of the normal coins veiled in this module
        let total_veiled_coins = cast_u32_to_u64_amount(150);
        assert!(total_veiled_coins<coin::FakeMoney>() == total_veiled_coins, 1);

        // Transfer `v = 50` of these veiled coins to the recipient
        let amount_val = ristretto255::new_scalar_from_u32(50);
        let amount_rand = ristretto255::random_scalar();

        // This will be the new balance `b' = b - 50 = 100` left at the `sender`, that we need to do a range proof for
        let new_balance_rand = ristretto255::scalar_neg(&amount_rand);
        let new_balance_val = ristretto255::new_scalar_from_u32(100);
        let (new_balance_range_proof, new_balance_ct) = bulletproofs::prove_range_elgamal(
            &new_balance_val, &new_balance_rand, &sender_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
        println(b"Computed range proof over the `sender`'s new balance");

        // Compute a range proof over the commitment to `v` and encrypt it under the `sender`'s PK
        let (amount_val_range_proof, withdraw_ct) = bulletproofs::prove_range_elgamal(
            &amount_val, &amount_rand, &sender_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
        println(b"Computed range proof over the transferred amount");

        // Register the `recipient` for receiving veiled coins
        let (_, recipient_pk) = generate_elgamal_keypair();
        register<coin::FakeMoney>(&recipient, elgamal::pubkey_to_bytes(&recipient_pk));
        println(b"Registered the `recipient` to receive veiled coins");
        // TODO: this throws an invariant violation
        //print(&recipient);

        // Encrypt the transfered amount `v` under the `recipient`'s PK
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(
            &amount_val, &amount_rand, &recipient_pk);

       let amount_comm = pedersen::new_commitment_for_bulletproof(&amount_val, &amount_rand);
       let new_balance_comm = pedersen::new_commitment_for_bulletproof(&new_balance_val, &new_balance_rand);
       println(b"Computed commitments to the amount to transfer and the sender's updated balance");

        // Prove that the two encryptions of `v` are to the same value
        let sigma_proof = sigma_protocol_prove<coin::FakeMoney>(
            &sender_pk, &recipient_pk, &withdraw_ct, &deposit_ct, &new_balance_ct, &new_balance_comm, &amount_comm, &amount_rand, &amount_val, &new_balance_rand, &new_balance_val);
        let sigma_proof_bytes = serialize_sigma_proof<coin::FakeMoney>(&sigma_proof);
        println(b"Created sigma protocol proof");

        // Sanity check veiled balances
        assert!(verify_opened_balance<coin::FakeMoney>(signer::address_of(&sender), 150, &scalar_zero(), &sender_pk), 1);
        assert!(verify_opened_balance<coin::FakeMoney>(signer::address_of(&recipient), 0, &scalar_zero(), &recipient_pk), 1);

        // Execute the veiled transaction: no one will be able to tell 50 coins are being transferred.
        fully_veiled_transfer<coin::FakeMoney>(
            &sender,
            signer::address_of(&recipient),
            elgamal::ciphertext_to_bytes(&withdraw_ct),
            elgamal::ciphertext_to_bytes(&deposit_ct),
            pedersen::commitment_to_bytes(&new_balance_comm),
            pedersen::commitment_to_bytes(&amount_comm),
            bulletproofs::range_proof_to_bytes(&new_balance_range_proof),
            bulletproofs::range_proof_to_bytes(&amount_val_range_proof),
            sigma_proof_bytes);
        println(b"Transferred veiled coins");

        // Sanity check veiled balances
        assert!(verify_opened_balance<coin::FakeMoney>(signer::address_of(&sender), 100, &new_balance_rand, &sender_pk), 1);
        assert!(verify_opened_balance<coin::FakeMoney>(signer::address_of(&recipient), 50, &amount_rand, &recipient_pk), 1);

        assert!(total_veiled_coins<coin::FakeMoney>() == total_veiled_coins, 1);

        // Drain the whole remaining balance of the sender
        let new_new_balance_val = ristretto255::new_scalar_from_u32(0);

        // `unveil` doesn't change the randomness, so we reuse the `new_balance_rand` randomness from before
        let (new_new_balance_range_proof, _) = bulletproofs::prove_range_elgamal(
            &new_new_balance_val, &new_balance_rand, &sender_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        // Unveil all coins of the `sender`
        unveil<coin::FakeMoney>(
            &sender, 100, bulletproofs::range_proof_to_bytes(&new_new_balance_range_proof));
        println(b"Unveiled all 100 coins from the `sender`'s veiled balance");

        let total_veiled_coins = cast_u32_to_u64_amount(50);
        assert!(total_veiled_coins<coin::FakeMoney>() == total_veiled_coins, 1);

        // Sanity check veiled balances
        assert!(verify_opened_balance<coin::FakeMoney>(signer::address_of(&sender), 0, &new_balance_rand, &sender_pk), 1);
        assert!(verify_opened_balance<coin::FakeMoney>(signer::address_of(&recipient), 50, &amount_rand, &recipient_pk), 1);
    }
}
