/// **WARNING:** This is an **experimental, proof-of-concept** module! It is *NOT* production-ready and it will likely
/// lead to loss of funds if used (or misused).
///
/// This module provides a veiled coin type, denoted `VeiledCoin<T>` that hides the value/denomination of a coin.
/// Importantly, although veiled transactions hide the amount of coins sent they still leak the sender and recipient.
///
/// ## How to use veiled coins
///
/// This module allows users to "register" a veiled account for any pre-existing `aptos_framework::Coin` type `T` via
/// the `register` entry function. For this, an encryption public key will need to be given as input, under which
/// the registered user's veiled balance will be encrypted.
///
/// Once Alice registers a veiled account for `T`, she can call `veil` with any public amount `a` of `T` coins
/// and add them to her veiled balance. Note that these coins will not be properly veiled yet, since they were withdrawn
/// from a public balance, which leaks their value.
///
/// (Alternatively, another user can initialize Alice's veiled balance by calling `veil_to`.)
///
/// Suppose Bob also registers and veils `b` of his own coins of type `T`.
///
/// Now Alice can use `fully_veiled_transfer` to send to Bob a secret amount `v` of coins from her veiled balance.
/// This will, for the first time, properly hide both Alice's and Bob's veiled balance.
/// The only information that an attacker (e.g., an Aptos validator) learns, is that Alice transferred an unknown amount
/// `v` to Bob (including $v=0$), and as a result Alice's veiled balance is in a range [a-v, a] and Bob's veiled balance
/// is in [b, b+v]`.
///
/// As more veiled transfers occur between more veiled accounts, the uncertainity on the balance of each account becomes
/// larger and larger.
///
/// Lastly, users can easily withdraw veiled coins back into their public balance via `unveil`. Or, they can withdraw
/// publicly into someone else's public balance via `unveil_to`.
///
/// ## Terminology
///
/// 1. *Veiled coin*: a coin whose value is secret; i.e., it is encrypted under the owner's public key.
///
/// 2. *Veiled amount*: any amount that is secret because it was encrypted under some public key.
/// 3. *Committed amount*: any amount that is secret because it was committed to (rather than encrypted).
///
/// 4. *Veiled transaction*: a transaction that hides its amount transferred; i.e., a transaction whose amount is veiled.
///
/// 5. *Veiled balance*: unlike a normal balance, a veiled balance is secret; i.e., it is encrypted under the account's
///    public key.
///
/// 6. *ZKRP*: zero-knowledge range proofs; one of the key cryptographic ingredient in veiled coins which ensures users
///    can withdraw secretely from their veiled balance without over-withdrawing.
///
/// ## Limitations
///
/// **WARNING:** This module is **experimental**! It is *NOT* production-ready. Specifically:
///
///  1. Deploying this module will likely lead to lost funds.
///  2. This module has not been cryptographically-audited.
///  3. The current implementation is vulnerable to _front-running attacks_ as described in the Zether paper [BAZB20].
///  4. There is no integration with wallet software which, for veiled accounts, must maintain an additional ElGamal
///    encryption keypair.
///  5. There is no support for rotating the ElGamal encryption public key of a veiled account.
///
/// ## Veiled coin amounts as truncated `u32`'s
///
/// Veiled coin amounts must be specified as `u32`'s rather than `u64`'s as would be typical for normal coins in the
/// Aptos framework. This is because coin amounts must be encrypted with an *efficient*, additively-homomorphic encryption
/// scheme. Currently, our best candidate is ElGamal encryption in the exponent, which can only decrypt values around
/// 32 bits or slightly larger.
///
/// Specifically, veiled coin amounts are restricted to be 32 bits and can be cast to a normal 64-bit coin value by
/// setting the leftmost and rightmost 16 bits to zero and the "middle" 32 bits to be the veiled coin bits.
///
/// This gives veiled amounts ~10 bits for specifying ~3 decimals and ~22 bits for specifying whole amounts, which
/// limits veiled balances and veiled transfers to around 4 million coins. (See `coin.move` for how a normal 64-bit coin
/// value gets interpreted as a decimal number.)
///
/// In order to convert a `u32` veiled coin amount to a normal `u64` coin amount, we have to shift it left by 16 bits.
///
/// ```
/// u64 normal coin amount format:
/// [ left    || middle  || right ]
/// [ 63 - 32 || 31 - 16 || 15 - 0]
///
/// u32 veiled coin amount format; we take the middle 32 bits from the u64 format above and store them in a u32:
/// [ middle ]
/// [ 31 - 0 ]
/// ```
///
/// Recall that: A coin has a *decimal precision* $d$ (e.g., for `AptosCoin`, $d = 8$; see `initialize` in
/// `aptos_coin.move`). This precision $d$ is used when displaying a `u64` amount, by dividing the amount by $10^d$.
/// For example, if the precision $d = 2$, then a `u64` amount of 505 coins displays as 5.05 coins.
///
/// For veiled coins, we can easily display a `u32` `Coin<T>` amount $v$ by:
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
/// the resource account can be used to transfer out the normal from its coin store. Transferring out a coin like this
/// requires a `signer` for the resource account, which the `veiled_coin` module can obtain via a `SignerCapability`.
///
/// ## References
///
/// [BAZB20] Zether: Towards Privacy in a Smart Contract World; by Bunz, Benedikt and Agrawal, Shashank and Zamani,
/// Mahdi and Boneh, Dan; in Financial Cryptography and Data Security; 2020
module aptos_experimental::veiled_coin {
    use std::error;
    use std::option::Option;
    use std::signer;
    use std::vector;

    use aptos_std::ristretto255;
    use aptos_std::ristretto255_bulletproofs as bulletproofs;
    use aptos_std::ristretto255_bulletproofs::RangeProof;
    use aptos_std::ristretto255_elgamal as elgamal;
    use aptos_std::ristretto255_pedersen as pedersen;
    #[test_only]
    use aptos_std::ristretto255::Scalar;

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event;

    use aptos_experimental::helpers;
    use aptos_experimental::sigma_protos;

    //
    // Errors
    //

    /// The range proof system does not support proofs for any number \in [0, 2^{32})
    const ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE: u64 = 1;

    /// A range proof failed to verify.
    const ERANGE_PROOF_VERIFICATION_FAILED: u64 = 2;

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

    /// The `NUM_LEAST_SIGNIFICANT_BITS_REMOVED` and `NUM_MOST_SIGNIFICANT_BITS_REMOVED` constants need to sum to 32 (bits).
    const EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT: u64 = 8;

    /// Non-specific internal error (see source code)
    const EINTERNAL_ERROR: u64 = 9;

    //
    // Constants
    //

    /// The maximum number of bits used to represent a coin's value.
    const MAX_BITS_IN_VEILED_COIN_VALUE: u64 = 32;

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
    const VEILED_COIN_BULLETPROOFS_DST: vector<u8> = b"AptosVeiledCoin/BulletproofRangeProof";

    //
    // Structs
    //

    /// A holder of a specific coin type and its associated event handles.
    /// These are kept in a single resource to ensure locality of data.
    struct VeiledCoinStore<phantom CoinType> has key {
        /// A ElGamal ciphertext of a value $v \in [0, 2^{32})$, an invariant that is enforced throughout the code.
        veiled_balance: elgamal::CompressedCiphertext,
        pk: elgamal::CompressedPubkey
    }

    #[event]
    /// Event emitted when some amount of veiled coins were deposited into an account.
    struct Deposit has drop, store {
        // We cannot leak any information about how much has been deposited.
        user: address
    }

    #[event]
    /// Event emitted when some amount of veiled coins were withdrawn from an account.
    struct Withdraw has drop, store {
        // We cannot leak any information about how much has been withdrawn.
        user: address
    }

    /// Holds an `account::SignerCapability` for the resource account created when initializing this module. This
    /// resource account houses a `coin::CoinStore<T>` for every type of coin `T` that is veiled.
    struct VeiledCoinMinter has store, key {
        signer_cap: account::SignerCapability
    }

    /// Main structure representing a coin in an account's custody.
    struct VeiledCoin<phantom CoinType> {
        /// ElGamal ciphertext which encrypts the number of coins $v \in [0, 2^{32})$. This $[0, 2^{32})$ range invariant
        /// is enforced throughout the code via Bulletproof-based ZK range proofs.
        veiled_amount: elgamal::Ciphertext
    }

    //
    // Structs for cryptographic proofs
    //

    /// A cryptographic proof that ensures correctness of a veiled-to-veiled coin transfer.
    struct TransferProof has drop {
        sigma_proof: sigma_protos::TransferSubproof,
        zkrp_new_balance: RangeProof,
        zkrp_amount: RangeProof
    }

    /// A cryptographic proof that ensures correctness of a veiled-to-*unveiled* coin transfer.
    struct WithdrawalProof has drop {
        sigma_proof: sigma_protos::WithdrawalSubproof,
        zkrp_new_balance: RangeProof
    }

    //
    // Module initialization, done only once when this module is first published on the blockchain
    //

    /// Initializes a so-called "resource" account which will maintain a `coin::CoinStore<T>` resource for all `Coin<T>`'s
    /// that have been converted into a `VeiledCoin<T>`.
    fun init_module(deployer: &signer) {
        assert!(
            bulletproofs::get_max_range_bits() >= MAX_BITS_IN_VEILED_COIN_VALUE,
            error::internal(ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE)
        );

        assert!(
            NUM_LEAST_SIGNIFICANT_BITS_REMOVED + NUM_MOST_SIGNIFICANT_BITS_REMOVED
                == 32,
            error::internal(EU64_COIN_AMOUNT_CLAMPING_IS_INCORRECT)
        );

        // Create the resource account. This will allow this module to later obtain a `signer` for this account and
        // transfer `Coin<T>`'s into its `CoinStore<T>` before minting a `VeiledCoin<T>`.
        let (_resource, signer_cap) =
            account::create_resource_account(deployer, vector::empty());

        move_to(deployer, VeiledCoinMinter { signer_cap })
    }

    //
    // Entry functions
    //

    /// Initializes a veiled account for the specified `user` such that their balance is encrypted under public key `pk`.
    /// Importantly, the user's wallet must retain their corresponding secret key.
    public entry fun register<CoinType>(user: &signer, pk: vector<u8>) {
        let pk = elgamal::new_pubkey_from_bytes(pk);
        register_internal<CoinType>(user, pk.extract());
    }

    /// Sends a *public* `amount` of normal coins from `sender` to the `recipient`'s veiled balance.
    ///
    /// **WARNING:** This function *leaks* the transferred `amount`, since it is given as a public input.
    public entry fun veil_to<CoinType>(
        sender: &signer, recipient: address, amount: u32
    ) acquires VeiledCoinMinter, VeiledCoinStore {
        let c = coin::withdraw<CoinType>(sender, cast_u32_to_u64_amount(amount));

        let vc = veiled_mint_from_coin(c);

        veiled_deposit<CoinType>(recipient, vc)
    }

    /// Like `veil_to`, except `owner` is both the sender and the recipient.
    ///
    /// This function can be used by the `owner` to initialize his veiled balance to a *public* value.
    ///
    /// **WARNING:** The initialized balance is *leaked*, since its initialized `amount` is public here.
    public entry fun veil<CoinType>(
        owner: &signer, amount: u32
    ) acquires VeiledCoinMinter, VeiledCoinStore {
        veil_to<CoinType>(owner, signer::address_of(owner), amount)
    }

    /// Takes a *public* `amount` of `VeiledCoin<CoinType>` coins from `sender`, unwraps them to a `coin::Coin<CoinType>`,
    /// and sends them to `recipient`. Maintains secrecy of `sender`'s new balance.
    ///
    /// Requires a ZK range proof on the new balance of the sender, to ensure the sender has enough money to send.
    /// No ZK range proof is necessary for the `amount`, which is given as a public `u32` value.
    ///
    /// **WARNING:** This *leaks* the transferred `amount`, since it is a public `u32` argument.
    public entry fun unveil_to<CoinType>(
        sender: &signer,
        recipient: address,
        amount: u32,
        comm_new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        withdraw_subproof: vector<u8>
    ) acquires VeiledCoinStore, VeiledCoinMinter {
        // Deserialize all the proofs into their proper Move structs
        let comm_new_balance = pedersen::new_commitment_from_bytes(comm_new_balance);
        assert!(
            comm_new_balance.is_some(),
            error::invalid_argument(EDESERIALIZATION_FAILED)
        );

        let sigma_proof =
            sigma_protos::deserialize_withdrawal_subproof(withdraw_subproof);
        assert!(
            std::option::is_some(&sigma_proof),
            error::invalid_argument(EDESERIALIZATION_FAILED)
        );

        let comm_new_balance = comm_new_balance.extract();
        let zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);

        let withdrawal_proof = WithdrawalProof {
            sigma_proof: std::option::extract(&mut sigma_proof),
            zkrp_new_balance
        };

        // Do the actual work
        unveil_to_internal<CoinType>(
            sender,
            recipient,
            amount,
            comm_new_balance,
            withdrawal_proof
        );
    }

    /// Like `unveil_to`, except the `sender` is also the recipient.
    public entry fun unveil<CoinType>(
        sender: &signer,
        amount: u32,
        comm_new_balance: vector<u8>,
        zkrp_new_balance: vector<u8>,
        withdraw_subproof: vector<u8>
    ) acquires VeiledCoinStore, VeiledCoinMinter {
        unveil_to<CoinType>(
            sender,
            signer::address_of(sender),
            amount,
            comm_new_balance,
            zkrp_new_balance,
            withdraw_subproof
        )
    }

    /// Sends a *veiled* amount from `sender` to `recipient`. After this call, the veiled balances of both the `sender`
    /// and the `recipient` remain (or become) secret.
    ///
    /// The sent amount always remains secret; It is encrypted both under the sender's PK (in `withdraw_ct`) & under the
    /// recipient's PK (in `deposit_ct`) using the *same* ElGamal randomness, so as to allow for efficiently updating both
    /// the sender's & recipient's veiled balances. It is also committed under `comm_amount`, so as to allow for a ZK
    /// range proof.
    ///
    /// Requires a `TransferProof`; i.e.:
    /// 1. A range proof `zkrp_new_balance` on the new balance of the sender, to ensure the sender has enough money to
    ///    send.
    /// 2. A range proof `zkrp_amount` on the transferred amount in `comm_amount`, to ensure the sender won't create
    ///    coins out of thin air.
    /// 3. A $\Sigma$-protocol proof `transfer_subproof` which proves that 'withdraw_ct' encrypts the same veiled amount
    ///    as in 'deposit_ct' (with the same randomness) and as in `comm_amount`.
    public entry fun fully_veiled_transfer<CoinType>(
        sender: &signer,
        recipient: address,
        withdraw_ct: vector<u8>,
        deposit_ct: vector<u8>,
        comm_new_balance: vector<u8>,
        comm_amount: vector<u8>,
        zkrp_new_balance: vector<u8>,
        zkrp_amount: vector<u8>,
        transfer_subproof: vector<u8>
    ) acquires VeiledCoinStore {
        // Deserialize everything into their proper Move structs
        let veiled_withdraw_amount = elgamal::new_ciphertext_from_bytes(withdraw_ct);
        assert!(
            veiled_withdraw_amount.is_some(),
            error::invalid_argument(EDESERIALIZATION_FAILED)
        );

        let veiled_deposit_amount = elgamal::new_ciphertext_from_bytes(deposit_ct);
        assert!(
            veiled_deposit_amount.is_some(),
            error::invalid_argument(EDESERIALIZATION_FAILED)
        );

        let comm_new_balance = pedersen::new_commitment_from_bytes(comm_new_balance);
        assert!(
            comm_new_balance.is_some(),
            error::invalid_argument(EDESERIALIZATION_FAILED)
        );

        let comm_amount = pedersen::new_commitment_from_bytes(comm_amount);
        assert!(
            comm_amount.is_some(), error::invalid_argument(EDESERIALIZATION_FAILED)
        );

        let transfer_subproof =
            sigma_protos::deserialize_transfer_subproof(transfer_subproof);
        assert!(
            std::option::is_some(&transfer_subproof),
            error::invalid_argument(EDESERIALIZATION_FAILED)
        );

        let transfer_proof = TransferProof {
            zkrp_new_balance: bulletproofs::range_proof_from_bytes(zkrp_new_balance),
            zkrp_amount: bulletproofs::range_proof_from_bytes(zkrp_amount),
            sigma_proof: std::option::extract(&mut transfer_subproof)
        };

        // Do the actual work
        fully_veiled_transfer_internal<CoinType>(
            sender,
            recipient,
            veiled_withdraw_amount.extract(),
            veiled_deposit_amount.extract(),
            comm_new_balance.extract(),
            comm_amount.extract(),
            &transfer_proof
        )
    }

    //
    // Public utility functions, for accessing state and converting u32 veiled coin amounts to u64 normal coin amounts.
    //

    /// Clamps a `u64` normal public amount to a `u32` to-be-veiled amount.
    ///
    /// WARNING: Precision is lost here (see "Veiled coin amounts as truncated `u32`'s" in the top-level comments)
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
    public fun veiled_balance<CoinType>(
        owner: address
    ): elgamal::CompressedCiphertext acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(owner),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED)
        );

        borrow_global<VeiledCoinStore<CoinType>>(owner).veiled_balance
    }

    /// Given an address `addr`, returns the ElGamal encryption public key associated with that address
    public fun encryption_public_key<CoinType>(
        addr: address
    ): elgamal::CompressedPubkey acquires VeiledCoinStore {
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

    /// Returns the domain separation tag (DST) for constructing Bulletproof-based range proofs in this module.
    public fun get_veiled_coin_bulletproofs_dst(): vector<u8> {
        VEILED_COIN_BULLETPROOFS_DST
    }

    /// Returns the maximum # of bits used to represent a veiled coin amount. Might differ than the 64 bits used to
    /// represent normal `aptos_framework::coin::Coin` values.
    public fun get_max_bits_in_veiled_coin_value(): u64 {
        MAX_BITS_IN_VEILED_COIN_VALUE
    }

    //
    // Public functions that modify veiled balances/accounts/coins
    // (These could be made private, but we leave them public since they might be helpful to other contracts building
    //  efficiently on top of veiled coins.)
    //

    /// Like `register`, but the public key has been parsed in a type-safe struct.
    /// TODO: Do we want to require a PoK of the SK here?
    public fun register_internal<CoinType>(
        user: &signer, pk: elgamal::CompressedPubkey
    ) {
        let account_addr = signer::address_of(user);
        assert!(
            !has_veiled_coin_store<CoinType>(account_addr),
            error::already_exists(EVEILED_COIN_STORE_ALREADY_PUBLISHED)
        );

        // Note: There is no way to find an ElGamal SK such that the `(0_G, 0_G)` ciphertext below decrypts to a non-zero
        // value. We'd need to have `(r * G, v * G + r * pk) = (0_G, 0_G)`, which implies `r = 0` for any choice of PK/SK.
        // Thus, we must have `v * G = 0_G`, which implies `v = 0`.

        let coin_store = VeiledCoinStore<CoinType> {
            veiled_balance: helpers::get_veiled_balance_zero_ciphertext(),
            pk
        };
        move_to(user, coin_store);
    }

    /// Deposits a veiled `coin` at address `to_addr`.
    public fun veiled_deposit<CoinType>(
        to_addr: address, coin: VeiledCoin<CoinType>
    ) acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(to_addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED)
        );

        let veiled_coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(to_addr);

        // Fetch the veiled balance
        let veiled_balance =
            elgamal::decompress_ciphertext(&veiled_coin_store.veiled_balance);

        // Add the veiled amount to the veiled balance (leverages the homomorphism of the encryption scheme)
        elgamal::ciphertext_add_assign(&mut veiled_balance, &coin.veiled_amount);

        // Update the veiled balance
        veiled_coin_store.veiled_balance = elgamal::compress_ciphertext(&veiled_balance);

        // Make sure the veiled coin is dropped so it cannot be double spent
        let VeiledCoin<CoinType> { veiled_amount: _ } = coin;

        // Once successful, emit an event that a veiled deposit occurred.
        event::emit(Deposit { user: to_addr });
    }

    /// Like `unveil_to`, except the proofs have been deserialized into type-safe structs.
    public fun unveil_to_internal<CoinType>(
        sender: &signer,
        recipient: address,
        amount: u32,
        comm_new_balance: pedersen::Commitment,
        withdrawal_proof: WithdrawalProof
    ) acquires VeiledCoinStore, VeiledCoinMinter {
        let addr = signer::address_of(sender);
        assert!(
            has_veiled_coin_store<CoinType>(addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED)
        );

        // Fetch the sender's ElGamal encryption public key
        let sender_pk = encryption_public_key<CoinType>(addr);

        // Fetch the sender's veiled balance
        let veiled_coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(addr);
        let veiled_balance =
            elgamal::decompress_ciphertext(&veiled_coin_store.veiled_balance);

        // Create a (not-yet-secure) encryption of `amount`, since `amount` is a public argument here.
        let scalar_amount = ristretto255::new_scalar_from_u32(amount);

        // Verify that `comm_new_balance` is a commitment to the remaing balance after withdrawing `amount`.
        sigma_protos::verify_withdrawal_subproof(
            &sender_pk,
            &veiled_balance,
            &comm_new_balance,
            &scalar_amount,
            &withdrawal_proof.sigma_proof
        );

        // Verify a ZK range proof on `comm_new_balance` (and thus on the remaining `veiled_balance`)
        verify_range_proofs(
            &comm_new_balance,
            &withdrawal_proof.zkrp_new_balance,
            &std::option::none(),
            &std::option::none()
        );

        let veiled_amount = elgamal::new_ciphertext_no_randomness(&scalar_amount);

        // Withdraw `amount` from the veiled balance (leverages the homomorphism of the encryption scheme.)
        elgamal::ciphertext_sub_assign(&mut veiled_balance, &veiled_amount);

        // Update the veiled balance to reflect the veiled withdrawal
        veiled_coin_store.veiled_balance = elgamal::compress_ciphertext(&veiled_balance);

        // Emit event to indicate a veiled withdrawal occurred
        event::emit(Withdraw { user: addr });

        // Withdraw normal `Coin`'s from the resource account and deposit them in the recipient's
        let c =
            coin::withdraw(
                &get_resource_account_signer(), cast_u32_to_u64_amount(amount)
            );

        coin::deposit<CoinType>(recipient, c);
    }

    /// Like `fully_veiled_transfer`, except the ciphertext and proofs have been deserialized into type-safe structs.
    public fun fully_veiled_transfer_internal<CoinType>(
        sender: &signer,
        recipient_addr: address,
        veiled_withdraw_amount: elgamal::Ciphertext,
        veiled_deposit_amount: elgamal::Ciphertext,
        comm_new_balance: pedersen::Commitment,
        comm_amount: pedersen::Commitment,
        transfer_proof: &TransferProof
    ) acquires VeiledCoinStore {
        let sender_addr = signer::address_of(sender);

        let sender_pk = encryption_public_key<CoinType>(sender_addr);
        let recipient_pk = encryption_public_key<CoinType>(recipient_addr);

        // Note: The `encryption_public_key` call from above already asserts that `sender_addr` has a coin store.
        let sender_veiled_coin_store =
            borrow_global_mut<VeiledCoinStore<CoinType>>(sender_addr);

        // Fetch the veiled balance of the veiled account
        let veiled_balance =
            elgamal::decompress_ciphertext(&sender_veiled_coin_store.veiled_balance);

        // Checks that `veiled_withdraw_amount` and `veiled_deposit_amount` encrypt the same amount of coins, under the
        // sender and recipient's PKs. Also checks this amount is committed inside `comm_amount`. Also, checks that the
        // new balance encrypted in `veiled_balance` is committed in `comm_new_balance`.
        sigma_protos::verify_transfer_subproof(
            &sender_pk,
            &recipient_pk,
            &veiled_withdraw_amount,
            &veiled_deposit_amount,
            &comm_amount,
            &comm_new_balance,
            &veiled_balance,
            &transfer_proof.sigma_proof
        );

        // Update the account's veiled balance by homomorphically subtracting the veiled amount from the veiled balance.
        elgamal::ciphertext_sub_assign(&mut veiled_balance, &veiled_withdraw_amount);

        // Verifies range proofs on the transferred amount and the remaining balance
        verify_range_proofs(
            &comm_new_balance,
            &transfer_proof.zkrp_new_balance,
            &std::option::some(comm_amount),
            &std::option::some(transfer_proof.zkrp_amount)
        );

        // Update the veiled balance to reflect the veiled withdrawal
        sender_veiled_coin_store.veiled_balance = elgamal::compress_ciphertext(
            &veiled_balance
        );

        // Once everything succeeds, emit an event to indicate a veiled withdrawal occurred
        event::emit(Withdraw { user: sender_addr });

        // Create a new veiled coin for the recipient.
        let vc = VeiledCoin<CoinType> { veiled_amount: veiled_deposit_amount };

        // Deposits `veiled_deposit_amount` into the recipient's account
        // (Note, if this aborts, the whole transaction aborts, so we do not need to worry about atomicity.)
        veiled_deposit(recipient_addr, vc);
    }

    /// Verifies range proofs on the remaining balance of an account committed in `comm_new_balance` and, optionally, on
    /// the transferred amount committed inside `comm_amount`.
    public fun verify_range_proofs(
        comm_new_balance: &pedersen::Commitment,
        zkrp_new_balance: &RangeProof,
        comm_amount: &Option<pedersen::Commitment>,
        zkrp_amount: &Option<RangeProof>
    ) {
        // Let `amount` denote the amount committed in `comm_amount` and `new_bal` the balance committed in `comm_new_balance`.
        //
        // This function checks if it is possible to withdraw a veiled `amount` from a veiled `bal`, obtaining a new
        // veiled balance `new_bal = bal - amount`. This function is used to maintains a key safety invariant throughout
        // the veild coin code: i.e., that every account has `new_bal \in [0, 2^{32})`.
        //
        // This invariant is enforced as follows:
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
        // When the caller of this function created the `comm_amount` from a public `u32` value, it is guaranteed that
        // condition (2) from above holds, so no range proof is necessary. This happens when withdrawing a public
        // amount from a veiled balance via `unveil_to` or `unveil`.

        // Checks that the remaining balance is >= 0; i.e., range condition (3)
        assert!(
            bulletproofs::verify_range_proof_pedersen(
                comm_new_balance,
                zkrp_new_balance,
                MAX_BITS_IN_VEILED_COIN_VALUE,
                VEILED_COIN_BULLETPROOFS_DST
            ),
            error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
        );

        // Checks that the transferred amount is in range (when this amount did not originate from a public amount); i.e., range condition (2)
        if (zkrp_amount.is_some()) {
            assert!(
                bulletproofs::verify_range_proof_pedersen(
                    comm_amount.borrow(),
                    zkrp_amount.borrow(),
                    MAX_BITS_IN_VEILED_COIN_VALUE,
                    VEILED_COIN_BULLETPROOFS_DST
                ),
                error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
            );
        };
    }

    //
    // Private functions.
    //

    /// Returns a signer for the resource account storing all the normal coins that have been veiled.
    fun get_resource_account_signer(): signer acquires VeiledCoinMinter {
        account::create_signer_with_capability(
            &borrow_global<VeiledCoinMinter>(@aptos_experimental).signer_cap
        )
    }

    /// Mints a veiled coin from a normal coin, shelving the normal coin into the resource account's coin store.
    ///
    /// **WARNING:** Fundamentally, there is no way to hide the value of the coin being minted here.
    fun veiled_mint_from_coin<CoinType>(
        c: Coin<CoinType>
    ): VeiledCoin<CoinType> acquires VeiledCoinMinter {
        // If there is no `coin::CoinStore<CoinType>` in the resource account, create one.
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

        // Paranoid check: assert that the u64 coin value had only its middle 32 bits set (should be the case
        // because the caller should have withdrawn a u32 amount, but enforcing this here anyway).
        assert!(
            cast_u32_to_u64_amount(value_u32) == value_u64,
            error::internal(EINTERNAL_ERROR)
        );

        // Deposit a normal coin into the resource account...
        coin::deposit(rsrc_acc_addr, c);

        // ...and mint a veiled coin, which is backed by the normal coin
        VeiledCoin<CoinType> {
            veiled_amount: helpers::public_amount_to_veiled_balance(value_u32)
        }
    }

    //
    // Test-only functions
    //

    #[test_only]
    /// Returns true if the balance at address `owner` equals `value`.
    /// Requires the ElGamal encryption randomness `r` and public key `pk` as auxiliary inputs.
    public fun verify_opened_balance<CoinType>(
        owner: address,
        value: u32,
        r: &Scalar,
        pk: &elgamal::CompressedPubkey
    ): bool acquires VeiledCoinStore {
        // compute the expected encrypted balance
        let value = ristretto255::new_scalar_from_u32(value);
        let expected_ct = elgamal::new_ciphertext_with_basepoint(&value, r, pk);

        // get the actual encrypted balance
        let actual_ct = elgamal::decompress_ciphertext(&veiled_balance<CoinType>(owner));

        elgamal::ciphertext_equals(&actual_ct, &expected_ct)
    }

    #[test_only]
    /// So we can call this from `veiled_coin_tests.move`.
    public fun init_module_for_testing(deployer: &signer) {
        init_module(deployer)
    }

    //
    // Unit tests are available in `veiled_coin_tests.move`.
    //
}
