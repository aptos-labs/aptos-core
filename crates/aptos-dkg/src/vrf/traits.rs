//! Notes: Unlike PVSS, we do NOT want to use a generic unweighted-to-weighted VRF transformation,
//! since we have a more optimized transformation for some VRF schemes (e.g., BLS and [GJM+21e]).
//!
//! As a result, we only define weighted VRF traits here.
//!
//! TODO: Could let \alpha, \beta be derived deterministically by hashing the SecretKeyShare, to avoid adding another layer of complexity for persisting them on disk.
//! TODO: Two approaches to deal with the pubkey augmentation problem:
//! 1. *Approach #1:* Each validator only derives their own SecretKeyShare and PubKeyShare from their
//!    PVSS share. Then, there is a special code path for the first run of the threshold VRF
//!    aggregation protocol.
//!
//!   - They will not fetch the other validator's PubKeyShare's, since they have not been augmented with
//!     the ElGamal-like encryptions of the SK.
//!
//!   - When receiving the first VUF share (and augmented PubKeyShare) from some other validator $i$,
//!     they will verify both and, if this succeeds, cache the PubKeyShare.
//!     TODO: This allows validator $i$ to equivocate about their PubKeyShare though. Not sure if this will be a problem.
//!     TODO: Maybe the proposer can include all the pubkeys of everyone that has contributed to the VUF
//!     successfully? The included pubkeys can be verified against the PVSS pubkeys. But that will
//!     leave some validators out, whouse pubkeys will need to be updated during a future VUF aggregation.
//!
//!   - (How will they know this is validator $i$'s PubKeyShare? Either from the consensus signatures
//!     or by using validator $i$'s PVSS PubKeyShare as the ground truth against which they run
//!     `VUF.PubkeyVerify`.)
//!
//! 2. *Approach #2*: Each validator derives their own SecretKeyShare and PubKeyShare as above, but
//!    also the other validator's "incomplete" PubKeyShare.
//!
//!    - When receiving a VUF sigshare from validator $i$, they check if $i$'s PubKeyShare is incomplete or not.
//!      If incomplete, they take the slow path for verification & update $i$'s PubKeyShare if successful.
//!      Otherwise, they take the fast path using $i$'s complete PubKeyShare.
//!      TODO: What if there's an inconsistency between the sigshare type received and they cached pubkey type?
//!      (If the share is "augmented", and the PK is or isn't, verification can still happen.
//!       But if the share is not augmented and the PK is not augmented either, verification will fail.)

pub trait PublicParameters {

}

pub trait SignatureShare : Clone {

}

pub trait Signature {
    type Evaluation: Evaluation;

    fn derive(&self) -> Self::Evaluation;
}

// TODO: Should use `Reconstructable` trait for aggregating shares into a signature

pub trait Evaluation {
    /// Converts a VRF evaluation into a vector of bytes.
    fn to_random_bytes(&self, num_bytes: usize) -> Vec<u8>;
}

pub trait SecretKey {
    type Signature: Signature;

    /// Produces a VUF signature (which can be converted into a VRF evaluation by applying a random oracle over it).
    fn sign(&self) -> Self::Signature;
}

pub trait SecretKeyShare {
    type SignatureShare: SignatureShare;

    fn sign(&self) -> Self::SignatureShare;
}

pub trait PubKeyShare {
    // TODO:
    fn signature_share_verify();

    /// Verifies the pubkey itself;
    /// TODO: we cannot have a two-state object though: verified/unverified... it'll be messy
    /// TODO: should we move this verification share_verify? But then it would need to modify the PKShare object to indicate it's been verified
    fn verify();
}

pub trait PubKey {
    /// Verifies the final aggregated signature. After, anyone can call `Signature::derive` on it.
    fn aggregate_signature_verify();
}
