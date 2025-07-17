use std::{marker::PhantomData, ops::Mul};
use crate::{group::{Fr, G1Affine, G1Projective}, shared::algebra::multi_point_eval::multi_point_eval_naive};

use ark_ff::FftField;
use ark_poly::{domain::DomainCoeff, EvaluationDomain, Radix2EvaluationDomain};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator as _};
use serde::{de::Visitor, ser::{SerializeSeq as _, SerializeStruct as _}, Deserialize, Serialize};
use crate::shared::ark_serialize::*;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};

use super::multi_point_eval::multi_point_eval;

// TODO have a better error-handling story. Currently there are a lot of assert_eq! which 
// should be replaced with either compile-time guarantees on array sizes or with Results.


/// To efficiently evaluate a Circulant matrix of size `n x n` over an input, 
/// a FFT-friendly subset of a field of size `n` is required. This struct
/// represents that subset. Following the terminology in Arkworks, we call this
/// subset a "domain".
#[derive(Debug, Clone)]
pub struct CirculantDomain<F: FftField> {
    fft_domain: Radix2EvaluationDomain<F>,
}

#[derive(Debug, Clone)]
pub struct PreparedInput<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> {
    pub y: Vec<T>,
    _phantom: PhantomData<F>,
}

impl<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> PreparedInput<F, T> {
    pub fn new(y: Vec<T>) -> Self {
        Self {
            y,
            _phantom: PhantomData
        }
    }
}

impl<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> Serialize for PreparedInput<F, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut seq = serializer.serialize_seq(Some(self.y.len()))?;        
        for elem in &self.y {
            let mut bytes = vec![];
            elem.serialize_with_mode(&mut bytes, Compress::Yes).map_err(serde::ser::Error::custom)?;
            seq.serialize_element(&bytes)?;
        }
        seq.end()
    }
}

struct PreparedInputVisitor<F: FftField, T: DomainCoeff<F> + CanonicalDeserialize> {
    pd: PhantomData<T>,
    pd2: PhantomData<F>,
}
impl<F: FftField, T: DomainCoeff<F> + CanonicalDeserialize> PreparedInputVisitor<F, T> {
    fn new() -> Self { Self { pd: PhantomData, pd2: PhantomData } }
}

impl<'de, F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> Visitor<'de> for PreparedInputVisitor<F, T> {
    type Value = PreparedInput<F, T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a struct of type PreparedInput")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut y : Vec<T> = vec![];
        while let Some(value) = seq.next_element::<Vec<u8>>()? {
            let value_t = T::deserialize_with_mode(value.as_slice(), Compress::Yes, Validate::Yes)
                .map_err(serde::de::Error::custom)?;
            y.push(value_t);
        }
        Ok(PreparedInput::new(y))
    }
}

impl<'de, F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>  Deserialize<'de> for PreparedInput<F, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        deserializer.deserialize_seq(PreparedInputVisitor::new())
    }
}


impl<F: FftField> CirculantDomain<F> {
    /// Create a new CirculantDomain of the specified `dimension`, which supports
    /// evaluating circulants of size `dimension x dimension`.
    pub fn new(dimension: usize) -> Option<Self> {
        Some(Self {
            fft_domain: Radix2EvaluationDomain::new(dimension)?
        })

    }

    pub fn dimension(&self) -> usize { self.fft_domain.size() }

    /// Evaluate a circulant matrix given by the vector `circulant`, on an input
    /// `input`. 
    ///
    /// A circulant matrix 
    /// ```
    /// ┌       ┐
    /// │ a c b │
    /// │ b a c │
    /// │ c b a │
    /// └       ┘
    /// ```
    /// is represented by a vector
    /// ```
    /// ┌       ┐
    /// │ a b c │
    /// └       ┘
    /// ```
    ///
    /// and the logic for efficient evaluation is explained here:
    /// https://alinush.github.io/2020/03/19/multiplying-a-vector-by-a-toeplitz-matrix.html#multiplying-a-circulant-matrix-by-a-vector
    pub fn eval<T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>(
        &self,
        circulant: &[F], 
        input: &[T],
    ) -> Vec<T> {
        assert_eq!(circulant.len(), input.len());
        assert_eq!(circulant.len(), self.dimension());

        let prepared_input = self.prepare_input(input);
        self.eval_prepared(circulant, &prepared_input)
    }

    /// One of the steps of efficient circulant matrix evaluation is an FFT on the input. Assuming you
    /// are going to evaluate multiple circulant matrices on the same input, it makes sense to do
    /// this step ahead of time. This function does this input preparation, taking an input vector
    /// `input` and outputting a [`PreparedInput`] which stores the result of this FFT.
    pub fn prepare_input<T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>(
        &self,
        input: &[T],
    ) -> PreparedInput<F, T> {
        let y = self.fft_domain.fft(input);
        PreparedInput::new(y)
    }

    /// Evaluate a circulant matrix given by the vector `circulant`, on a prepared input 
    /// `prepared_input`.
    pub fn eval_prepared<T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>(
        &self,
        circulant: &[F], 
        prepared_input: &PreparedInput<F, T>,
    ) -> Vec<T> {
        assert_eq!(circulant.len(), prepared_input.y.len());
        assert_eq!(circulant.len(), self.dimension());

        let v = self.fft_domain.fft(circulant);
        
        // Hadamard product of y and v
        // Would like to write
        // let u : Vec<T> = zip(y, v).map(|(yi, vi)| yi * vi).collect();
        // but looks like DomainCoeff only is required to implement MulAssign<F> and not Mul<F>...
        // so have to do something uglier:
        let mut u : Vec<T> = prepared_input.y.clone();
        u.par_iter_mut().zip(v.par_iter()).for_each(|(u, v)| {
            *u *= *v;
        });

        self.fft_domain.ifft(&u)
    }
}


/// To efficiently evaluate a Toeplitz matrix of size `n x n` over an input, 
/// a FFT-friendly subset of a field of size `2 * n` is required. This struct
/// represents that subset. Following the terminology in Arkworks, we call this
/// subset a "domain".
#[derive(Debug, Clone)]
pub struct ToeplitzDomain<F: FftField + Sized> {
    pub circulant_domain: CirculantDomain<F>
}

impl<F: FftField + Sized> ToeplitzDomain<F> {

    /// dimension is `n` where Toeplitz matrix is size `n x n`, and thus the vector 
    /// representation of the matrix is of size `2*n - 1`.
    pub fn new(dimension: usize) -> Option<Self> {
        Some(Self {
            circulant_domain: CirculantDomain::new(2 * dimension)?
        })
    }

    pub fn dimension(&self) -> usize {
        self.circulant_domain.dimension() / 2
    }

    /// Convert a Toeplitz matrix `T` into a corresponding circulant matrix
    /// `C` such that `Tx = C[x 0^dimension]^T`.
    ///
    /// A Toeplitz matrix
    /// ```
    /// ┌       ┐
    /// │ c b a │
    /// │ d c b │
    /// │ e d c │
    /// └       ┘
    /// ```
    /// is represented by a vector
    /// ```
    /// ┌           ┐
    /// │ a b c d e │
    /// └           ┘
    /// ```
    /// and is converted into a circulant matrix of twice 
    /// the dimension:
    /// ```
    /// ┌              ┐
    /// │ c b a  c e d │
    /// │ d c b  a c e │
    /// │ e d c  b a c │
    /// │              │
    /// │ c e d  c b a │
    /// │ a c e  d c b │
    /// │ b a c  e d c │
    /// └              ┘
    /// ```
    /// which is represented by a vector
    /// ```
    /// ┌             ┐
    /// │ c d e c a b │
    /// └             ┘
    /// ```
    /// and where the evaluation identity above holds. Why this is true is explained in
    /// [https://alinush.github.io/2020/03/19/multiplying-a-vector-by-a-toeplitz-matrix.html#multiplying-a-toeplitz-matrix-by-a-vector](https://alinush.github.io/2020/03/19/multiplying-a-vector-by-a-toeplitz-matrix.html#multiplying-a-toeplitz-matrix-by-a-vector).
    pub fn toeplitz_to_circulant(&self, toeplitz: &[F]) -> Vec<F> {
        assert_eq!(toeplitz.len() + 1,  self.circulant_domain.dimension());
        let middle_element = vec![toeplitz[toeplitz.len() / 2]];
        let beginning = Vec::from(&toeplitz[0 .. toeplitz.len() / 2]);
        let end = Vec::from(&toeplitz[toeplitz.len() / 2 + 1 ..]);
        

        let circulant : Vec<F> = middle_element.clone()
            .into_iter()
            .chain(end)
            .chain(middle_element)
            .chain(beginning)
            .collect();

        assert_eq!(circulant.len(), self.circulant_domain.dimension());

        circulant
    }

    /// Evaluate a Toeplitz matrix given by the vector `toeplitz`, on an input
    /// `input`. 
    ///
    /// This is done efficiently by first converting the Toeplitz matrix to a circulant matrix 
    /// in a way that preserves evaluation (see [`Self::toeplitz_to_circulant()`]), and then by 
    /// evaluating the circulant matrix efficiently using [`CirculantDomain::eval()`].
    pub fn eval<T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>(
        &self,
        toeplitz: &[F], 
        input: &[T],
    ) -> Vec<T> {
        assert_eq!(toeplitz.len() + 1,  self.circulant_domain.dimension());
        assert_eq!(2 * input.len(),  self.circulant_domain.dimension());

        let prepared_input = self.prepare_input(input);

        self.eval_prepared(toeplitz, &prepared_input)
    }

    /// Prepare an input `input` in a similar way to [`CirculantDomain::prepare_input`].
    pub fn prepare_input<T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>(
        &self,
        input: &[T],
    ) -> PreparedInput<F, T> {

        let expanded_input : Vec<T> = Vec::from(input)
            .into_iter()
            .chain(vec![T::zero(); input.len()])
            .collect();

        self.circulant_domain.prepare_input(&expanded_input)
    }

    /// Evaluate a Toeplitz matrix given by the vector `toeplitz`, on a prepared input 
    /// `prepared_input`.
    pub fn eval_prepared<T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>(
        &self,
        toeplitz: &[F], 
        prepared_input: &PreparedInput<F, T>,
    ) -> Vec<T> {
        assert_eq!(toeplitz.len() + 1,  self.circulant_domain.dimension());
        assert_eq!(prepared_input.y.len(),  self.circulant_domain.dimension());

        Vec::from(&self.circulant_domain.eval_prepared(
            &self.toeplitz_to_circulant(toeplitz),
            prepared_input)[..self.dimension()])
    }
}



/// Encapsulates the [`ToeplitzDomain`] and a FFT evaluation domain required for running the FK
/// algorithm.
#[derive(Debug, Clone)]
pub struct FKDomain<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> {
    pub toeplitz_domain: ToeplitzDomain<F>,
    pub fft_domain: Radix2EvaluationDomain<F>,
    pub prepared_toeplitz_inputs: Vec<PreparedInput<F, T>>,
}


impl<F: FftField, T: DomainCoeff<F> + Mul<F, Output = T> + CanonicalSerialize + CanonicalDeserialize> FKDomain<F, T> {
    
    pub fn new(max_poly_degree: usize, eval_domain_size: usize, tau_powers: Vec<Vec<T>>) -> Option<Self> {
        let toeplitz_domain = ToeplitzDomain::new(max_poly_degree)?;

        let tau_powers_reversed : Vec<Vec<T>> = 
            tau_powers.into_iter().map(|tau_powers_for_round|
            Vec::from(tau_powers_for_round).into_iter().rev().collect()
            ).collect();
        let prepared_toeplitz_inputs = 
            tau_powers_reversed.into_iter().map(|tau_powers_reversed_for_round|
                toeplitz_domain.prepare_input(&tau_powers_reversed_for_round[1..])
            ).collect();

        Some(Self { 
            toeplitz_domain,
            fft_domain: Radix2EvaluationDomain::new(eval_domain_size)?,
            prepared_toeplitz_inputs
        })
    }


    /// Compute the corresponding Toeplitz matrix for a polynomial `f`, as explained here:
    /// [https://alinush.github.io/feist-khovratovich#computing-the-h_j--gh_jtau-commitments](https://alinush.github.io/feist-khovratovich#computing-the-h_j--gh_jtau-commitments)
    /// TODO explain this diagram
    /// ```
    /// ┌        ┐     ┌            ┐   ┌  ┐
    /// │ H (X)  │     │ f   f   f  │   │ 2│
    /// │  1     │     │  3   2   1 │   │X │
    /// │        │     │            │   │  │
    /// │ H (X)  │  =  │ 0   f   f  │ ● │ 1│
    /// │  2     │     │      3   2 │   │X │
    /// │        │     │            │   │  │
    /// │ H (X)  │     │ 0   0   f  │   │ 0│
    /// │  3     │     │          3 │   │X │
    /// └        ┘     └            ┘   └  ┘
    /// ```
    pub fn toeplitz_for_poly(&self, f: &[F]) -> Vec<F> {
        let toeplitz : Vec<F> = Vec::from(&f[1..])
            .into_iter()
            .chain(vec![F::zero(); f.len() - 2])
            .collect();

        assert_eq!(toeplitz.len(), self.toeplitz_domain.dimension() * 2 - 1);

        toeplitz
    }

    /// Compute the evaluation proofs for a KZG commitment of a polynomial `f`, committed to under
    /// `tau_powers`, on the FFT domain encapsulated by this [`FKDomain`].
    pub fn eval_proofs_at_roots_of_unity(&self, f: &[F], round: usize) -> Vec<T> {
        // f.len() = (degree of f) + 1. Degree of f should be equal to the toeplitz domain
        // dimension.
        let mut f = Vec::from(f);
        f.extend(std::iter::repeat(F::zero()).take(self.toeplitz_domain.dimension() + 1 - f.len()));
        assert_eq!(self.toeplitz_domain.dimension(), f.len() - 1);

        let h_term_commitments 
            = self.toeplitz_domain.eval_prepared(
                &self.toeplitz_for_poly(&f),
                // The Toeplitz matrix is only evaluated on the powers up to max_poly_degree - 1,
                // since the H_j(X) polynomials have degree at most that
                &self.prepared_toeplitz_inputs[round]
            );

        self.fft_domain.fft(&h_term_commitments)
    }

    pub fn eval_proofs_at_x_coords(&self, f: &[F], x_coords: &[F], round: usize) -> Vec<T> {
        // f.len() = (degree of f) + 1. Degree of f should be equal to the toeplitz domain
        // dimension.
        let mut f = Vec::from(f);
        f.extend(std::iter::repeat(F::zero()).take(self.toeplitz_domain.dimension() + 1 - f.len()));
        assert_eq!(self.toeplitz_domain.dimension(), f.len() - 1);

        let h_term_commitments 
            = self.toeplitz_domain.eval_prepared(
                &self.toeplitz_for_poly(&f),
                // The Toeplitz matrix is only evaluated on the powers up to max_poly_degree - 1,
                // since the H_j(X) polynomials have degree at most that
                &self.prepared_toeplitz_inputs[round]
            );

        multi_point_eval(&h_term_commitments, &x_coords)
    }

}

impl<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> Serialize for FKDomain<F, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let mut state = serializer.serialize_struct("FKDomain", 3)?;
        state.serialize_field("toeplitz_domain_dimension", &self.toeplitz_domain.dimension())?;
        state.serialize_field("fft_domain_size", &self.fft_domain.size)?;
        state.serialize_field("prepared_toeplitz_inputs", &self.prepared_toeplitz_inputs)?;
        state.end()
    }
}

impl<'de, F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> Deserialize<'de> for FKDomain<F, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        todo!()
    }
}

pub trait EPTest {
    fn eval_proofs_at_x_coords_alt(&self, f: &[Fr], x_coords: &[Fr], round: usize) -> Vec<G1Affine>;
}

use ark_std::Zero;

impl EPTest for FKDomain<Fr, G1Projective> {
    fn eval_proofs_at_x_coords_alt(&self, f: &[Fr], x_coords: &[Fr], round: usize) -> Vec<G1Affine> {
        // f.len() = (degree of f) + 1. Degree of f should be equal to the toeplitz domain
        // dimension.
        let mut f = Vec::from(f);
        f.extend(std::iter::repeat(Fr::zero()).take(self.toeplitz_domain.dimension() + 1 - f.len()));
        assert_eq!(self.toeplitz_domain.dimension(), f.len() - 1);

        let h_term_commitments 
            = self.toeplitz_domain.eval_prepared(
                &self.toeplitz_for_poly(&f),
                // The Toeplitz matrix is only evaluated on the powers up to max_poly_degree - 1,
                // since the H_j(X) polynomials have degree at most that
                &self.prepared_toeplitz_inputs[round]
            );

        multi_point_eval_naive(
            &h_term_commitments.into_iter().map(|g| G1Affine::from(g)).collect::<Vec<G1Affine>>(), 
            &x_coords)
    }
}

#[cfg(test)]
mod tests {
    use ark_ec::{AffineRepr as _, ScalarMul as _};
    use ark_ec::{pairing::Pairing, VariableBaseMSM, PrimeGroup};
    use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial, Polynomial};
    use ark_poly::EvaluationDomain;
    use ark_std::{rand::thread_rng, UniformRand, One};
    use crate::group::{PairingSetting, Fr, G1Affine, G1Projective, G2Projective, G2Affine};

    use crate::shared::digest::DigestKey;

    use super::FKDomain;



    #[test]
    fn compute_eval_proofs_at_roots_of_unity() {
        // TODO right now the only supported (max) polynomial degrees are powers of 2. Maybe I should change
        // that for better usability?
        for poly_degree_exp in 1..4 {
            let poly_degree = 2usize.pow(poly_degree_exp);
            let mut rng = thread_rng();

            let tau = Fr::rand(&mut rng);

            let mut tau_powers_fr = vec![Fr::one()];
            let mut cur = tau.clone();
            for _ in 0..poly_degree {
                tau_powers_fr.push(cur);
                cur *= &tau;
            }

            let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
                (0..(poly_degree + 1)).map(|_| Fr::rand(&mut rng)).collect()
            );

            let tau_powers_g1 = G1Projective::from(G1Affine::generator()).batch_mul(&tau_powers_fr);
            let tau_powers_g1_projective : Vec<Vec<G1Projective>> = vec! [ tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect() ];
            let tau_g2 : G2Affine = (G2Affine::generator() * tau).into();

            let fk_domain = FKDomain::new(poly_degree, poly_degree, tau_powers_g1_projective).unwrap();
            let commitment : G1Affine = G1Projective::msm(&tau_powers_g1, &poly.coeffs).unwrap().into();

            let evaluation_proofs = fk_domain.eval_proofs_at_roots_of_unity(&poly.coeffs, 0);

            for i in 0..poly_degree {
                let lhs = PairingSetting::pairing(
                    G1Projective::from(commitment)
                    - 
                    (G1Projective::generator() * poly.evaluate(&fk_domain.fft_domain.element(i))), G2Projective::generator()
                );
                let rhs = PairingSetting::pairing(
                    evaluation_proofs[i], 
                    G2Projective::from(tau_g2)
                    - 
                    (G2Projective::generator() * fk_domain.fft_domain.element(i))
                );
                assert_eq!(lhs, rhs);
            }
        }
    }

}
