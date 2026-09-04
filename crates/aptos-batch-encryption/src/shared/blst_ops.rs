// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Routes pairings and G1 MSMs through the blst library (hand-tuned ADX
//! assembly), which is substantially faster than arkworks' pure-Rust
//! arithmetic. Both libraries represent Fq identically (Montgomery form, six
//! little-endian limbs) and build the same Fq12 tower, so conversions are limb
//! moves and results are bit-identical to the arkworks equivalents.
//!
//! BLS12-381 only; if `crate::group` is switched to another curve this module
//! must be disabled.

use crate::{
    group::{Fr, G1Affine, G1Projective, PairingOutput},
    shared::algebra::{fk_algorithm::EvalRepr, multi_point_eval::powers_of},
};
use ark_bls12_381::{Fq, Fq12, Fq2, Fq6, G2Affine};
use ark_ec::{AffineRepr, CurveGroup as _};
use ark_ff::{BigInt, BigInteger, PrimeField, Zero};
use blst::{
    blst_final_exp, blst_fp, blst_fp12, blst_fp12_mul, blst_fp6, blst_miller_loop,
    blst_miller_loop_lines, blst_p1, blst_p1_add_or_double, blst_p1_affine, blst_p1_cneg,
    blst_p1_from_affine, blst_p1_is_equal, blst_p1_is_inf, blst_p1_mult, blst_p1_serialize,
    blst_p2_affine, blst_precompute_lines,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator as _};
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

// Both libraries hold an Fq as six little-endian 64-bit limbs in Montgomery
// form against the same modulus and the same R = 2^384, so the two
// representations are bit-identical and converting is a move, not arithmetic.
//
// The obvious spelling -- serialize to big-endian bytes and call
// `blst_fp_from_bendian` / `blst_bendian_from_fp` -- costs a byte swap plus a
// full Montgomery round trip in each direction, on 24 limbs per Fq12, on every
// pairing. `test_fq_conversion_matches_bendian` pins the equivalence.
fn fq_to_blst_fp(x: &Fq) -> blst_fp {
    blst_fp { l: x.0 .0 }
}

fn blst_fp_to_fq(fp: &blst_fp) -> Fq {
    // Already Montgomery-form and reduced, which is exactly `new_unchecked`'s
    // contract; `Fp::new` would multiply by R^2 and give a different element.
    Fq::new_unchecked(BigInt(fp.l))
}

fn g1_to_blst_affine(p: &G1Affine) -> blst_p1_affine {
    match p.xy() {
        Some((x, y)) => blst_p1_affine {
            x: fq_to_blst_fp(&x),
            y: fq_to_blst_fp(&y),
        },
        // blst represents the point at infinity as the all-zero affine point.
        None => blst_p1_affine::default(),
    }
}

fn g2_to_blst_affine(q: &G2Affine) -> blst_p2_affine {
    match q.xy() {
        Some((x, y)) => blst_p2_affine {
            x: blst::blst_fp2 {
                fp: [fq_to_blst_fp(&x.c0), fq_to_blst_fp(&x.c1)],
            },
            y: blst::blst_fp2 {
                fp: [fq_to_blst_fp(&y.c0), fq_to_blst_fp(&y.c1)],
            },
        },
        None => blst_p2_affine::default(),
    }
}

fn blst_p1_to_ark(p: &blst_p1) -> G1Projective {
    let mut bytes = [0u8; 96];
    unsafe { blst_p1_serialize(bytes.as_mut_ptr(), p) };
    if bytes[0] & 0x40 != 0 {
        return G1Projective::zero();
    }
    // Serialized form is big-endian x || y; the top three bits of byte 0 are
    // flag bits (all zero here since the point is uncompressed and finite).
    let x = Fq::from_be_bytes_mod_order(&bytes[..48]);
    let y = Fq::from_be_bytes_mod_order(&bytes[48..]);
    G1Affine::new_unchecked(x, y).into()
}

fn blst_fp12_to_ark(f: &blst_fp12) -> Fq12 {
    let fq6 = |i: usize| {
        let fq2 = |j: usize| {
            Fq2::new(
                blst_fp_to_fq(&f.fp6[i].fp2[j].fp[0]),
                blst_fp_to_fq(&f.fp6[i].fp2[j].fp[1]),
            )
        };
        Fq6::new(fq2(0), fq2(1), fq2(2))
    };
    Fq12::new(fq6(0), fq6(1))
}

/// Equivalent to `PairingSetting::pairing(p, q)`, computed with blst.
pub fn pairing(p: &G1Affine, q: &G2Affine) -> PairingOutput {
    multi_pairing(std::slice::from_ref(p), std::slice::from_ref(q))
}

/// Equivalent to `PairingSetting::multi_pairing(ps, qs)`, computed with blst.
pub fn multi_pairing(ps: &[G1Affine], qs: &[G2Affine]) -> PairingOutput {
    assert_eq!(ps.len(), qs.len());
    let mut acc: Option<blst_fp12> = None;
    for (p, q) in ps.iter().zip(qs) {
        if p.is_zero() || q.is_zero() {
            continue;
        }
        let p = g1_to_blst_affine(p);
        let q = g2_to_blst_affine(q);
        let mut ml = blst_fp12::default();
        unsafe { blst_miller_loop(&mut ml, &q, &p) };
        acc = Some(match acc {
            None => ml,
            Some(prev) => {
                let mut out = blst_fp12::default();
                unsafe { blst_fp12_mul(&mut out, &prev, &ml) };
                out
            },
        });
    }
    match acc {
        None => PairingOutput::zero(),
        Some(mut acc) => {
            unsafe { blst_final_exp(&mut acc, &acc) };
            ark_ec::pairing::PairingOutput(blst_fp12_to_ark(&acc))
        },
    }
}

/// The number of Miller-loop line coefficients blst precomputes for a G2 point.
/// Fixed by `blst.h`'s `blst_precompute_lines(blst_fp6 Qlines[68], ...)`: the
/// BLS12-381 Miller loop runs 63 doublings and 5 additions over the bits of `x`.
const N_LINES: usize = 68;

/// blst's precomputed Miller-loop line coefficients for a fixed G2 point -- the
/// analogue of arkworks' `G2Prepared`, and usable the same way: build it once
/// while the G2 point is known, then pair against many G1 points without
/// redoing the G2-side work.
///
/// Note that this is *not* interchangeable with `G2Prepared` despite holding
/// the same count and shape of coefficients (68 triples of Fq2). blst derives
/// its lines in Jacobian coordinates and arkworks in homogeneous projective, so
/// corresponding entries differ by per-step projective scalars; the Miller loop
/// only tolerates those because their product dies in the final exponentiation.
#[derive(Clone)]
pub struct G2Lines {
    lines: Box<[blst_fp6; N_LINES]>,
    infinity: bool,
}

impl G2Lines {
    pub fn new(q: &G2Affine) -> Self {
        let mut lines = Box::new([blst_fp6::default(); N_LINES]);
        if !q.is_zero() {
            let q = g2_to_blst_affine(q);
            unsafe { blst_precompute_lines(lines.as_mut_ptr(), &q) };
        }
        Self {
            lines,
            infinity: q.is_zero(),
        }
    }

    /// Equivalent to `pairing(p, q)` for the `q` these lines were built from,
    /// but skipping the G2-side line computation.
    pub fn pairing(&self, p: &G1Affine) -> PairingOutput {
        if self.infinity || p.is_zero() {
            return PairingOutput::zero();
        }
        let p = g1_to_blst_affine(p);
        let mut ml = blst_fp12::default();
        unsafe {
            blst_miller_loop_lines(&mut ml, self.lines.as_ptr(), &p);
            blst_final_exp(&mut ml, &ml);
        }
        ark_ec::pairing::PairingOutput(blst_fp12_to_ark(&ml))
    }
}

/// A blst-backed G1 point that implements `DomainCoeff<Fr>`, so arkworks'
/// generic (parallel) FFT routines run over blst point arithmetic instead of
/// arkworks' own. The all-zero `blst_p1` is the point at infinity.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct BlstG1(pub blst_p1);

impl BlstG1 {
    pub fn from_ark(p: &G1Affine) -> Self {
        let aff = g1_to_blst_affine(p);
        let mut out = blst_p1::default();
        unsafe { blst_p1_from_affine(&mut out, &aff) };
        Self(out)
    }

    pub fn to_ark(&self) -> G1Projective {
        blst_p1_to_ark(&self.0)
    }
}

/// Lets the Toeplitz/circulant evaluation in [`crate::shared::algebra::fk_algorithm`]
/// run its Hadamard product and group inverse FFT over blst points: the prepared
/// tau powers are converted in one batch up front, and everything downstream is
/// blst arithmetic.
impl EvalRepr<Fr, G1Projective> for BlstG1 {
    fn from_prepared(prepared: &[G1Projective]) -> Vec<Self> {
        // A single batch inversion for the whole conversion.
        G1Projective::normalize_batch(prepared)
            .iter()
            .map(Self::from_ark)
            .collect()
    }
}

impl PartialEq for BlstG1 {
    fn eq(&self, other: &Self) -> bool {
        unsafe { blst_p1_is_equal(&self.0, &other.0) }
    }
}

impl Eq for BlstG1 {}

impl AddAssign for BlstG1 {
    fn add_assign(&mut self, rhs: Self) {
        unsafe { blst_p1_add_or_double(&mut self.0, &self.0, &rhs.0) };
    }
}

impl Add for BlstG1 {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl SubAssign for BlstG1 {
    fn sub_assign(&mut self, rhs: Self) {
        let mut neg = rhs.0;
        unsafe {
            blst_p1_cneg(&mut neg, true);
            blst_p1_add_or_double(&mut self.0, &self.0, &neg);
        }
    }
}

impl Sub for BlstG1 {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

impl MulAssign<Fr> for BlstG1 {
    fn mul_assign(&mut self, rhs: Fr) {
        let bytes = rhs.into_bigint().to_bytes_le();
        unsafe { blst_p1_mult(&mut self.0, &self.0, bytes.as_ptr(), 255) };
    }
}

impl Mul<Fr> for BlstG1 {
    type Output = Self;

    fn mul(mut self, rhs: Fr) -> Self {
        self *= rhs;
        self
    }
}

impl Zero for BlstG1 {
    fn zero() -> Self {
        // z = 0 is blst's representation of the point at infinity.
        Self(blst_p1::default())
    }

    fn is_zero(&self) -> bool {
        unsafe { blst_p1_is_inf(&self.0) }
    }
}

/// Window size for `blst_p1s_mult_wbits`. Table memory is
/// `npoints * 2^(WBITS-1) * 96` bytes, i.e. ~1.5 MiB at 128 points.
const MSM_WBITS: usize = 8;

/// A precomputed blst window table for repeated single-threaded MSMs over the
/// same G1 bases. `msm` is deliberately single-threaded (unlike blst's
/// `p1_affines::mult`, which spins up its own thread pool per call) so that
/// callers can parallelize over many MSMs with rayon without oversubscribing.
pub struct G1MsmBases {
    table: Vec<blst_p1_affine>,
    len: usize,
}

impl G1MsmBases {
    pub fn new(bases: &[G1Affine]) -> Self {
        Self::from_blst(&bases.iter().map(BlstG1::from_ark).collect::<Vec<BlstG1>>())
    }

    pub fn from_blst(points: &[BlstG1]) -> Self {
        let n = points.len();
        let mut affine = vec![blst_p1_affine::default(); n];
        // `BlstG1` is a transparent newtype, so this slice is layout-compatible
        // with the `blst_p1[]` that blst expects.
        let point_ptrs = [points.as_ptr().cast::<blst_p1>(), std::ptr::null()];
        unsafe { blst::blst_p1s_to_affine(affine.as_mut_ptr(), point_ptrs.as_ptr(), n) };

        let table_len = unsafe { blst::blst_p1s_mult_wbits_precompute_sizeof(MSM_WBITS, n) }
            / std::mem::size_of::<blst_p1_affine>();
        let mut table = vec![blst_p1_affine::default(); table_len];
        // The table is one contiguous block of `table_len / n` entries per
        // point, so the precompute parallelizes cleanly across points.
        let row_len = table_len / n;
        use rayon::{iter::IndexedParallelIterator as _, slice::ParallelSliceMut as _};
        table
            .par_chunks_mut(row_len)
            .zip(affine.par_iter())
            .for_each(|(row, point)| {
                let ptrs = [point as *const blst_p1_affine, std::ptr::null()];
                unsafe {
                    blst::blst_p1s_mult_wbits_precompute(
                        row.as_mut_ptr(),
                        MSM_WBITS,
                        ptrs.as_ptr(),
                        1,
                    )
                };
            });

        Self { table, len: n }
    }

    /// Equivalent to `G1Projective::msm(bases, scalars)`, computed with blst
    /// on the calling thread only.
    pub fn msm(&self, scalars: &[Fr]) -> G1Projective {
        assert_eq!(scalars.len(), self.len);
        let mut scalar_bytes = Vec::with_capacity(32 * scalars.len());
        for s in scalars {
            scalar_bytes.extend_from_slice(&s.into_bigint().to_bytes_le());
        }
        let scalar_ptrs = [scalar_bytes.as_ptr(), std::ptr::null()];

        let scratch_len = unsafe { blst::blst_p1s_mult_wbits_scratch_sizeof(self.len) }
            / std::mem::size_of::<u64>();
        let mut scratch = vec![0u64; scratch_len];

        let mut ret = blst_p1::default();
        unsafe {
            blst::blst_p1s_mult_wbits(
                &mut ret,
                self.table.as_ptr(),
                MSM_WBITS,
                self.len,
                scalar_ptrs.as_ptr(),
                255,
                scratch.as_mut_ptr(),
            )
        };
        blst_p1_to_ark(&ret)
    }
}

/// Naive multi-point evaluation of the G1-polynomial `f` at each of
/// `x_coords`, i.e. one size-`f.len()` MSM per coordinate, all sharing one
/// blst Pippenger table.
pub fn multi_point_eval_naive(f: &[G1Affine], x_coords: &[Fr]) -> Vec<G1Projective> {
    multi_point_eval_naive_with_bases(&G1MsmBases::new(f), x_coords)
}

/// Same as [`multi_point_eval_naive`], but starting from an already-built
/// Pippenger table (e.g. from blst-native points, skipping ark conversions).
pub fn multi_point_eval_naive_with_bases(bases: &G1MsmBases, x_coords: &[Fr]) -> Vec<G1Projective> {
    x_coords
        .par_iter()
        .map(|x| bases.msm(&powers_of(x, bases.len)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::PairingSetting;
    use ark_ec::{pairing::Pairing as _, CurveGroup, VariableBaseMSM};
    use ark_std::{rand::thread_rng, UniformRand};

    /// The limb-move conversions must agree with the big-endian-bytes spelling
    /// they replaced. If a future arkworks or blst release changes either
    /// internal representation, this is what catches it.
    #[test]
    fn test_fq_conversion_matches_bendian() {
        use ark_ff::{BigInteger as _, PrimeField as _};

        let via_bendian_to_blst = |x: &Fq| {
            let bytes = x.into_bigint().to_bytes_be();
            let mut fp = blst_fp::default();
            unsafe { blst::blst_fp_from_bendian(&mut fp, bytes.as_ptr()) };
            fp
        };
        let via_bendian_to_ark = |fp: &blst_fp| {
            let mut bytes = [0u8; 48];
            unsafe { blst::blst_bendian_from_fp(bytes.as_mut_ptr(), fp) };
            Fq::from_be_bytes_mod_order(&bytes)
        };

        let mut rng = thread_rng();
        // Include the edge cases a random sample will never produce.
        let mut xs = vec![Fq::zero(), Fq::from(1u64), -Fq::from(1u64)];
        xs.extend((0..64).map(|_| Fq::rand(&mut rng)));

        for x in xs {
            let fp = fq_to_blst_fp(&x);
            assert_eq!(fp.l, via_bendian_to_blst(&x).l, "ark -> blst for {x}");
            assert_eq!(blst_fp_to_fq(&fp), via_bendian_to_ark(&fp), "blst -> ark");
            assert_eq!(blst_fp_to_fq(&fp), x, "round trip");
        }
    }

    #[test]
    fn test_pairing_matches_ark() {
        let mut rng = thread_rng();
        for _ in 0..3 {
            let p = G1Affine::rand(&mut rng);
            let q = G2Affine::rand(&mut rng);
            assert_eq!(pairing(&p, &q), PairingSetting::pairing(p, q));
        }
        assert_eq!(
            pairing(&G1Affine::zero(), &G2Affine::rand(&mut rng)),
            PairingSetting::pairing(G1Affine::zero(), G2Affine::rand(&mut rng))
        );
    }

    #[test]
    fn test_g2_lines_pairing_matches_ark() {
        let mut rng = thread_rng();
        for _ in 0..3 {
            let p = G1Affine::rand(&mut rng);
            let q = G2Affine::rand(&mut rng);
            assert_eq!(G2Lines::new(&q).pairing(&p), PairingSetting::pairing(p, q));
        }
        // Either argument at infinity gives the identity in Gt.
        let q = G2Affine::rand(&mut rng);
        assert!(G2Lines::new(&q).pairing(&G1Affine::zero()).is_zero());
        assert!(G2Lines::new(&G2Affine::zero())
            .pairing(&G1Affine::rand(&mut rng))
            .is_zero());
    }

    /// Are arkworks' `G2Prepared` line coefficients and blst's precomputed lines
    /// the same bytes? If they were, an existing `G2Prepared` could be handed
    /// straight to `blst_miller_loop_lines` and the two libraries' prepared
    /// forms would be interchangeable.
    ///
    /// They are not -- blst works in Jacobian coordinates and arkworks in
    /// homogeneous projective -- but that is a structural argument, so check it
    /// rather than assert it. Run with:
    ///   cargo test -p aptos-batch-encryption --release -- perf_probe --ignored --nocapture
    #[test]
    #[ignore]
    fn perf_probe_lines_layout() {
        use crate::group::G2Prepared;

        let mut rng = thread_rng();
        let q = G2Affine::rand(&mut rng);

        let ark = G2Prepared::from(q);
        let blst = G2Lines::new(&q);

        println!("ark ell_coeffs: {}", ark.ell_coeffs.len());
        println!("blst lines:     {}", N_LINES);

        let mut matching = 0usize;
        for (i, (c0, c1, c2)) in ark.ell_coeffs.iter().enumerate() {
            // Both sides store Fq as 6 little-endian u64 limbs in Montgomery
            // form with the same R and modulus, so the limbs are directly
            // comparable.
            let ark_limbs: Vec<[u64; 6]> = [c0, c1, c2]
                .iter()
                .flat_map(|c| [c.c0.0 .0, c.c1.0 .0])
                .collect();
            let blst_limbs: Vec<[u64; 6]> = (0..3)
                .flat_map(|j| {
                    let fp2 = blst.lines[i].fp2[j];
                    [fp2.fp[0].l, fp2.fp[1].l]
                })
                .collect();
            if ark_limbs == blst_limbs {
                matching += 1;
            }
        }
        println!("entries matching byte-for-byte: {}/{}", matching, N_LINES);

        // Whatever the layout, the two must agree on the pairing itself.
        let p = G1Affine::rand(&mut rng);
        assert_eq!(blst.pairing(&p), PairingSetting::pairing(p, q));
    }

    /// How much of a pairing is the G2-side line precompute? That is the work
    /// `prepare()` could absorb so that `decrypt()` does not have to.
    ///
    /// Timed for both libraries, because the split is what the two designs
    /// differ on: baseline stored an arkworks `G2Prepared`, so its precompute
    /// already sat in `prepare()`, while the blst port pairs from a `G2Affine`
    /// and pays for the lines inside `decrypt()`.
    #[test]
    #[ignore]
    fn perf_probe_lines_timing() {
        use crate::group::G2Prepared;
        use std::time::Instant;

        let mut rng = thread_rng();
        let p = G1Affine::rand(&mut rng);
        let q = G2Affine::rand(&mut rng);
        let lines = G2Lines::new(&q);
        let prepared = G2Prepared::from(q);

        // Warm up, and confirm every route agrees before timing them.
        let expected = PairingSetting::pairing(p, q);
        assert_eq!(pairing(&p, &q), expected);
        assert_eq!(lines.pairing(&p), expected);

        const N: u32 = 200;
        let time = |label: &str, f: &dyn Fn()| {
            let start = Instant::now();
            for _ in 0..N {
                f();
            }
            let ns = start.elapsed().as_nanos() as f64 / f64::from(N);
            println!("  {label:<30} {:>9.1} µs", ns / 1000.0);
            ns
        };

        // The decrypt half, spelled out: arkworks' `pairing()` prepares
        // internally, so the prepared path has to go through `multi_miller_loop`
        // and `final_exponentiation` by hand.
        let ark_from_prepared = || {
            let ml = PairingSetting::multi_miller_loop([p], [prepared.clone()]);
            PairingSetting::final_exponentiation(ml).unwrap()
        };
        assert_eq!(ark_from_prepared(), expected);

        println!("\narkworks");
        let ark_full = time("full pairing", &|| {
            let _ = std::hint::black_box(PairingSetting::pairing(p, q));
        });
        let ark_prep = time("G2Prepared::from  (prepare)", &|| {
            std::hint::black_box(G2Prepared::from(q));
        });
        let ark_rest = time("miller_loop + final_exp (decrypt)", &|| {
            let _ = std::hint::black_box(ark_from_prepared());
        });

        println!("\nblst");
        let blst_full = time("full pairing", &|| {
            let _ = std::hint::black_box(pairing(&p, &q));
        });
        let blst_prep = time("precompute_lines  (prepare)", &|| {
            std::hint::black_box(G2Lines::new(&q));
        });
        let blst_rest = time("miller_loop_lines + final_exp (decrypt)", &|| {
            let _ = std::hint::black_box(lines.pairing(&p));
        });

        println!(
            "\n{:<10} {:>10} {:>10} {:>10} {:>10}",
            "", "full", "prepare", "decrypt", "prep %"
        );
        let row = |label: &str, full: f64, prep: f64, rest: f64| {
            println!(
                "{label:<10} {:>9.1} {:>9.1} {:>9.1} {:>9.1}%",
                full / 1000.0,
                prep / 1000.0,
                rest / 1000.0,
                100.0 * prep / full
            );
        };
        row("arkworks", ark_full, ark_prep, ark_rest);
        row("blst", blst_full, blst_prep, blst_rest);
    }

    #[test]
    fn test_multi_pairing_matches_ark() {
        let mut rng = thread_rng();
        let ps: Vec<G1Affine> = (0..3).map(|_| G1Affine::rand(&mut rng)).collect();
        let qs: Vec<G2Affine> = (0..3).map(|_| G2Affine::rand(&mut rng)).collect();
        assert_eq!(
            multi_pairing(&ps, &qs),
            PairingSetting::multi_pairing(ps.clone(), qs.clone())
        );
    }

    #[test]
    fn test_msm_matches_ark() {
        let mut rng = thread_rng();
        let bases: Vec<G1Affine> = (0..64).map(|_| G1Affine::rand(&mut rng)).collect();
        let scalars: Vec<Fr> = (0..64).map(|_| Fr::rand(&mut rng)).collect();
        let expected = G1Projective::msm(&bases, &scalars).unwrap();
        let actual = G1MsmBases::new(&bases).msm(&scalars);
        assert_eq!(actual.into_affine(), expected.into_affine());
    }

    #[test]
    fn test_multi_point_eval_naive_matches_ark() {
        let mut rng = thread_rng();
        let f: Vec<G1Affine> = (0..16).map(|_| G1Affine::rand(&mut rng)).collect();
        let xs: Vec<Fr> = (0..16).map(|_| Fr::rand(&mut rng)).collect();
        let expected: Vec<G1Projective> =
            crate::shared::algebra::multi_point_eval::multi_point_eval_naive(&f, &xs);
        let actual = multi_point_eval_naive(&f, &xs);
        for (a, e) in actual.iter().zip(&expected) {
            assert_eq!(a.into_affine(), e.into_affine());
        }
    }
}
