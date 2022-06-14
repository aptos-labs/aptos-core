// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides APIs for Boneh-Lynn-Shacham (BLS) multi-signatures on top of Barreto-Lynn-Scott
//! BLS12-381 elliptic curves. This module wraps the [blst](https://github.com/supranational/blst)
//! library.
//!
//! More specifically, we build BLS multisignatures as described in [^BLS04], [^Bold03] but using
//! the proof-of-possession (PoP) scheme from [^RY07] to prevent rogue-key attacks [^MOR01] where
//! malicious signers adversarially pick their public keys in order to forge a multisignature.
//!
//! We implement the `Minimal-pubkey-size` variant from the BLS IETF draft standard [^bls-ietf-draft],
//! which puts the signatures in the group $\mathbb{G}_2$ and the public keys in $\mathbb{G}_1$. The
//! reasoning behind this choice is to minimize public key size, since public keys are posted on the
//! blockchain.
//!
//! # Overview of BLS multisignatures
//!
//! In a _normal signature scheme_, we have a single _signer_ who generates its own key-pair:
//! a _private-key_ and a corresponding _public key_. The signer can produce a _signature_ on a
//! _message_ `m` using its private-key. Any _verifier_ who has the public key can check that
//! the signature on `m` was produced by the signer.
//!
//! In a _multisignature scheme_, we have `n` signers. Each signer `i` has their own key-pair `(sk_i, pk_i)`.
//! Any subset of `k` signers can collaborate to produce a succinct signature on the same message `m`.
//! This is referred to as a _multisignature_ on `m`.
//!
//! Typically, the `k` signers first agree on the message `m` via some protocol (e.g., `m` is the
//! latest block header in a blockchain protocol). Then, each signer produces a _signature share_ `s_i`
//! on `m` using their own private key `sk_i`. After this, each signer `i` sends their signature
//! share `s_i` to an _aggregator_: a dedicated, untrusted party who is responsible for aggregating
//! the signature shares into the final multisignature. For example, one of the signers themselves
//! could be the aggregator.
//!
//! Lastly, the aggregator can proceed in two ways:
//!
//! 1. Pessimistically verify each signature share, discarding the invalid ones, and then aggregate
//!    the final multisignature.
//!
//! 2. Optimistically aggregate all signature shares, but verify the final multisignature at the end
//!    to ensure no bad signature shares were included. If the multisignature does not verify,
//!    revert to the pessimistic mode (or consider other approaches [^LM07]).
//!
//! Either way, the end result (assuming some of the signature shares were valid) will be a valid
//! multisignature on `m` which can be verified against an _aggregate public key_ of the involved
//! signers.
//!
//! Specifically, any verifier who knows the public keys of the signers whose shares were aggregated
//! into the multisignature, can first compute an _aggregate public key_ as a function of these
//! public keys and then verify the multisignature under this aggregate public key.
//!
//! Extremely important for security is that the verifier first ensure these public keys came with
//! valid proofs-of-possession (PoPs). Otherwise, multisignatures can be forged via _rogue-key attacks_
//! [^MOR01].
//!
//! # A note on subgroup checks
//!
//! This library was written so that users who know nothing about _small subgroup attacks_ need not
//! worry about them [^LL97], [^BCM+15e], as long as library users always verify a public key's
//! proof-of-possession (PoP) before aggregating it with other PKs or before verifying signatures
//! with it.
//!
//! Nonetheless, we still provide `group_check` methods for the `PublicKey` and `Signature` structs,
//! in case manual verification of subgroup membership is needed.
//!
//! # A note on domain separation tags (DSTs)
//!
//! Internal to this wrapper's implementation (and to the underlying blst library) is the careful
//! use of domain separation tags (DSTs) as per the BLS IETF draft standard [^bls-ietf-draft].
//!
//! Specifically, **when signing a message** `m`, instead of signing as `H(m)^sk`, where `sk` is the
//! secret key, the library actually signs as `H(sig_dst | m)^sk`, where `sig_dst` is a domain
//! separation tag for message signing.
//!
//! In contrast, **when computing a proof-of-possesion (PoP)**, instead of signing the public key as
//! `H(pk)^sk`, the  library actually signs as `H(sig_pop | pk)^sk`, where `sig_pop` is a domain
//! separation tag for signatures used during PoP creation.
//!
//! This way, we can clearly separate the message spaces of these two use cases of the secret key `sk`.
//!
//! # How to use this module to aggregate and verify multisignatures
//!
//! A typical use of the library would look as follows:
//!
//! ```
//! use std::iter::zip;
//! use aptos_crypto::test_utils::KeyPair;
//! use aptos_crypto::{bls12381, Signature, SigningKey, Uniform};
//! use aptos_crypto::bls12381::bls12381_keys::{PrivateKey, PublicKey};
//! use aptos_crypto::bls12381::ProofOfPossession;
//! use aptos_crypto_derive::{CryptoHasher, BCSCryptoHash};
//! use rand_core::OsRng;
//! use serde::{Serialize, Deserialize};
//!
//! // Each signer locally generates their own BLS key-pair with a proof-of-possesion (PoP).
//! // We simulate this here, by storing each signer's key-pair in a vector.
//! let mut rng = OsRng;
//!
//! let num_signers = 1000;
//!
//! let mut key_pairs = vec![];
//! let mut pops = vec![];
//! for _ in 0..num_signers {
//!     let kp = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);
//!     pops.push(ProofOfPossession::create_with_pubkey(&kp.private_key, &kp.public_key));
//!     // Alternatively, but slower, can choose not to provide the PK and have it computed inside
//!     // pops.push(ProofOfPossession::create(&kp.private_key));
//!     key_pairs.push(kp);
//! }
//!
//! // Any arbitrary struct can be signed as long as it is properly "derived"
//! #[derive(CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
//! struct Message(String);
//!
//! // Each signer then computes a signature share on a message. Again, we simulate using a vector.
//! let mut sigshares = vec![];
//! let message = Message("test".to_owned()); // b"some random message";
//! for kp in key_pairs.iter() {
//!     let sig = kp.private_key.sign(&message);
//!     sigshares.push(sig);
//! }
//!
//! // Then, an aggregator receives some of these signature shares and will attempt to aggregate
//! // them in a multisig. This aggregator can proceed _optimistically_ as follows:
//!
//! // First, when the aggregator boots up, it must verify that each signer's public key has a valid
//! // proof-of-possession (PoP)!
//!
//! ///////////////////////////////////////////////////////////////////////////////////////////////
//! // WARNING: The aggregator and any other party verifying multisigs or signature shares MUST  //
//! // verify *every* signer's PoP before verifying any signature or multisignature.             //
//! //                                                                                           //
//! //                  The importance of this step cannot be overstated!                        //
//! //                                                                                           //
//! ///////////////////////////////////////////////////////////////////////////////////////////////
//! for i in 0..pops.len() {
//!     assert!(pops[i].verify(&key_pairs[i].public_key).is_ok());
//! }
//!
//! // Second, now that the aggregator trusts the set of public keys, it can safely aggregate
//! // signature shares _optimistically_ into a multisignature which hopefully verifies. In this
//! // example, we assume the aggregator receives a signature share from every third signer (for simplicity).
//!
//! // Here, we simulate the aggregator receiving some signature shares.
//! let mut sigshares_received = vec![];
//! for sigshare in sigshares.into_iter().step_by(3) {
//!     sigshares_received.push(sigshare);
//! }
//!
//! // Here, the aggregator aggregates the received signature shares into a multisignature.
//! let multisig = bls12381::Signature::aggregate(sigshares_received.clone()).unwrap();
//!
//! // Third, the aggregator checks that the _optimistic_ aggregation from above succeeded by
//! // verifying the multisig. For this, the aggregator will need to know the public keys of the
//! // signers whose signature shares were aggregated, so that it can aggregate them.
//! let mut pubkeys_to_agg = vec![];
//! for kp in key_pairs.iter().step_by(3) {
//!     pubkeys_to_agg.push(&kp.public_key);
//! }
//!
//! let aggpk = PublicKey::aggregate(pubkeys_to_agg.clone()).unwrap();
//!
//! // Lastly, the aggregator checks the aggregated multisig verifies successfully.
//! assert!(multisig.verify(&message, &aggpk).is_ok());
//!
//! // If the multisig failed verification, the aggregator can individually verify each of the
//! // signature shares to identify which ones are invalid and exclude them. There are also optimized
//! // methods for identifying bad signature shares faster when their relative frequency is low [^LM07].
//! // However, we will not implement these yet.
//! for (sigshare, pk) in zip(sigshares_received, pubkeys_to_agg) {
//!     assert!(sigshare.verify(&message, &pk).is_ok());
//! }
//! ```
//!
//! References:
//!
//! [^bls-ietf-draft]: BLS Signatures; by D. Boneh, S. Gorbunov, R. Wahby, H. Wee, Z. Zhang; https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-bls-signature
//! [^Bold03]: Threshold Signatures, Multisignatures and Blind Signatures Based on the Gap-Diffie-Hellman-Group Signature Scheme; by Boldyreva, Alexandra; in PKC 2003; 2002
//! [^BLS04]: Short Signatures from the Weil Pairing; by Boneh, Dan and Lynn, Ben and Shacham, Hovav; in Journal of Cryptology; 2004; https://doi.org/10.1007/s00145-004-0314-9
//! [^BCM+15e] Subgroup security in pairing-based cryptography; by Paulo S.  L.  M.  Barreto and Craig Costello and Rafael Misoczki and Michael Naehrig and Geovandro C.  C.  F.  Pereira and Gustavo Zanon; in Cryptology ePrint Archive, Paper 2015/247; 2015; https://eprint.iacr.org/2015/247
//! [^LL97] A key recovery attack on discrete log-based schemes using a prime order subgroup; by Lim, Chae Hoon and Lee, Pil Joong; in Advances in Cryptology --- CRYPTO '97; 1997
//! [^LM07]: Finding Invalid Signatures in Pairing-Based Batches; by Law, Laurie and Matt, Brian J.; in Cryptography and Coding; 2007
//! [^MOR01]: Accountable-Subgroup Multisignatures: Extended Abstract; by Micali, Silvio and Ohta, Kazuo and Reyzin, Leonid; in Proceedings of the 8th ACM Conference on Computer and Communications Security; 2001; https://doi-org.libproxy.mit.edu/10.1145/501983.502017
//! [^RY07]: The Power of Proofs-of-Possession: Securing Multiparty Signatures against Rogue-Key Attacks; by Ristenpart, Thomas and Yilek, Scott; in Advances in Cryptology - EUROCRYPT 2007; 2007

/// Domain separation tag (DST) for hashing a message before signing it.
const DST_BLS_MULTISIG_IN_G2_WITH_POP: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

pub mod bls12381_keys;
pub mod bls12381_pop;
pub mod bls12381_sigs;

pub use bls12381_keys::{PrivateKey, PublicKey};
pub use bls12381_pop::ProofOfPossession;
pub use bls12381_sigs::Signature;
