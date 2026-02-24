// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::multi_point_eval::multi_point_eval;
use crate::shared::algebra::multi_point_eval::multi_point_eval_naive;
use aptos_crypto::arkworks::serialization::{ark_de, ark_se};
use ark_ec::VariableBaseMSM;
use ark_ff::FftField;
use ark_poly::{domain::DomainCoeff, EvaluationDomain, Radix2EvaluationDomain};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
    ParallelIterator as _,
};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct as _,
    Deserialize, Serialize,
};
use std::{fmt, marker::PhantomData, ops::Mul};

// TODO have a better error-handling story. Currently there are a lot of assert_eq! which
// should be replaced with either compile-time guarantees on array sizes or with Results.

/// To efficiently evaluate a Circulant matrix of size `n x n` over an input,
/// a FFT-friendly subset of a field of size `n` is required. This struct
/// represents that subset. Following the terminology in Arkworks, we call this
/// subset a "domain".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CirculantDomain<F: FftField> {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    fft_domain: Radix2EvaluationDomain<F>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedInput<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>
{
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub y: Vec<T>,
    _phantom: PhantomData<F>,
}

impl<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>
    PreparedInput<F, T>
{
    pub fn new(y: Vec<T>) -> Self {
        Self {
            y,
            _phantom: PhantomData,
        }
    }
}

impl<F: FftField> CirculantDomain<F> {
    /// Create a new CirculantDomain of the specified `dimension`, which supports
    /// evaluating circulants of size `dimension x dimension`.
    pub fn new(dimension: usize) -> Option<Self> {
        Some(Self {
            fft_domain: Radix2EvaluationDomain::new(dimension)?,
        })
    }

    pub fn dimension(&self) -> usize {
        self.fft_domain.size()
    }

    /// Evaluate a circulant matrix given by the vector `circulant`, on an input
    /// `input`.
    ///
    /// A circulant matrix
    /// ```txt
    /// ┌       ┐
    /// │ a c b │
    /// │ b a c │
    /// │ c b a │
    /// └       ┘
    /// ```
    /// is represented by a vector
    /// ```txt
    /// ┌       ┐
    /// │ a b c │
    /// └       ┘
    /// ```
    ///
    /// The circulant matrix `C` can be thought of as a polynomial `C(X) = a + bX + cX^2`. Consider
    /// some input vector `v = [v_0, v_1, v_2]^T`, which can also be thought of as representing
    /// a polynomial $v(X) = v_0 + v_1 X + v_2 X^2`. The evaluation `Cx` of `C` on input `x` is the
    /// same as computing the convolution (i.e. multiplication in $F[X]/(X^n - 1)$) of the two
    /// polynomials `C(X)` and `v(X)`. Convolution of two polynomials can be computed by
    /// elementwise multiplication in the evaluation domain. Thus, we can compute `Cx` by moving
    /// `C` and `x` to the eval domain via FFT, multiplying elementwise, and then using iFFT to
    /// move back to the coefficient domain.
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
        let mut u: Vec<T> = prepared_input.y.clone();
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToeplitzDomain<F: FftField + Sized> {
    pub circulant_domain: CirculantDomain<F>,
}

impl<F: FftField + Sized> ToeplitzDomain<F> {
    /// dimension is `n` where Toeplitz matrix is size `n x n`, and thus the vector
    /// representation of the matrix is of size `2*n - 1`.
    pub fn new(dimension: usize) -> Option<Self> {
        Some(Self {
            circulant_domain: CirculantDomain::new(2 * dimension)?,
        })
    }

    pub fn dimension(&self) -> usize {
        self.circulant_domain.dimension() / 2
    }

    /// Convert a Toeplitz matrix `T` into a corresponding circulant matrix
    /// `C` such that `Tx = C[x 0^dimension]^T`.
    ///
    /// A Toeplitz matrix
    /// ```txt
    /// ┌       ┐
    /// │ c b a │
    /// │ d c b │
    /// │ e d c │
    /// └       ┘
    /// ```
    /// is represented by a vector
    /// ```txt
    /// ┌           ┐
    /// │ a b c d e │
    /// └           ┘
    /// ```
    /// and is converted into a circulant matrix of twice
    /// the dimension:
    /// ```txt
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
    /// ```txt
    /// ┌             ┐
    /// │ c d e c a b │
    /// └             ┘
    /// ```
    /// and where the evaluation identity above holds. Why this is true is explained in
    /// [https://alinush.github.io/2020/03/19/multiplying-a-vector-by-a-toeplitz-matrix.html#multiplying-a-toeplitz-matrix-by-a-vector](https://alinush.github.io/2020/03/19/multiplying-a-vector-by-a-toeplitz-matrix.html#multiplying-a-toeplitz-matrix-by-a-vector).
    pub fn toeplitz_to_circulant(&self, toeplitz: &[F]) -> Vec<F> {
        assert_eq!(toeplitz.len() + 1, self.circulant_domain.dimension());
        let middle_element = vec![toeplitz[toeplitz.len() / 2]];
        let beginning = Vec::from(&toeplitz[0..toeplitz.len() / 2]);
        let end = Vec::from(&toeplitz[toeplitz.len() / 2 + 1..]);

        let circulant: Vec<F> = middle_element
            .clone()
            .into_iter()
            .chain(end)
            .chain(middle_element)
            .chain(beginning)
            .collect();

        debug_assert_eq!(circulant.len(), self.circulant_domain.dimension());

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
        assert_eq!(toeplitz.len() + 1, self.circulant_domain.dimension());
        assert_eq!(2 * input.len(), self.circulant_domain.dimension());

        let prepared_input = self.prepare_input(input);

        self.eval_prepared(toeplitz, &prepared_input)
    }

    /// Prepare an input `input` in a similar way to [`CirculantDomain::prepare_input`].
    pub fn prepare_input<T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>(
        &self,
        input: &[T],
    ) -> PreparedInput<F, T> {
        let expanded_input: Vec<T> = Vec::from(input)
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
        assert_eq!(toeplitz.len() + 1, self.circulant_domain.dimension());
        assert_eq!(prepared_input.y.len(), self.circulant_domain.dimension());

        Vec::from(
            &self
                .circulant_domain
                .eval_prepared(&self.toeplitz_to_circulant(toeplitz), prepared_input)
                [..self.dimension()],
        )
    }
}

/// Encapsulates the [`ToeplitzDomain`] and a FFT evaluation domain required for running the FK
/// algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FKDomain<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> {
    pub toeplitz_domain: ToeplitzDomain<F>,
    pub fft_domain: Radix2EvaluationDomain<F>,
    pub prepared_toeplitz_inputs: Vec<PreparedInput<F, T>>,
}

impl<
        F: FftField,
        T: DomainCoeff<F>
            + Mul<F, Output = T>
            + VariableBaseMSM<ScalarField = F>
            + CanonicalSerialize
            + CanonicalDeserialize,
    > FKDomain<F, T>
{
    pub fn new(
        max_poly_degree: usize,
        eval_domain_size: usize,
        tau_powers: Vec<Vec<T>>,
    ) -> Option<Self> {
        let toeplitz_domain = ToeplitzDomain::new(max_poly_degree)?;

        let tau_powers_reversed: Vec<Vec<T>> = tau_powers
            .into_iter()
            .map(|tau_powers_for_round| tau_powers_for_round.into_iter().rev().collect())
            .collect();

        let prepared_toeplitz_inputs = tau_powers_reversed
            .into_iter()
            .map(|tau_powers_reversed_for_round| {
                toeplitz_domain.prepare_input(&tau_powers_reversed_for_round[1..])
            })
            .collect();

        Some(Self {
            toeplitz_domain,
            fft_domain: Radix2EvaluationDomain::new(eval_domain_size)?,
            prepared_toeplitz_inputs,
        })
    }

    /// Compute the corresponding Toeplitz matrix for a polynomial `f`, as explained here:
    /// [https://alinush.github.io/feist-khovratovich#computing-the-h_j--gh_jtau-commitments](https://alinush.github.io/feist-khovratovich#computing-the-h_j--gh_jtau-commitments)
    /// TODO explain this diagram
    /// ```txt
    /// [        ]     [            ]   [  ]
    /// [ H (X)  ]     [ f   f   f  ]   [ 2]
    /// [  1     ]     [  3   2   1 ]   [X ]
    /// [        ]     [            ]   [  ]
    /// [ H (X)  ]  =  [ 0   f   f  ] * [ 1]
    /// [  2     ]     [      3   2 ]   [X ]
    /// [        ]     [            ]   [  ]
    /// [ H (X)  ]     [ 0   0   f  ]   [ 0]
    /// [  3     ]     [          3 ]   [X ]
    /// [        ]     [            ]   [  ]
    /// ```
    pub fn toeplitz_for_poly(&self, f: &[F]) -> Vec<F> {
        let toeplitz: Vec<F> = Vec::from(&f[1..])
            .into_iter()
            .chain(vec![F::zero(); f.len() - 2])
            .collect();

        debug_assert_eq!(toeplitz.len(), self.toeplitz_domain.dimension() * 2 - 1);

        toeplitz
    }

    fn compute_h_term_commitments(&self, f: &[F], round: usize) -> Vec<T> {
        let mut f = Vec::from(f);
        f.extend(std::iter::repeat_n(
            F::zero(),
            self.toeplitz_domain.dimension() + 1 - f.len(),
        ));
        // f.len() = (degree of f) + 1. Degree of f should be equal to the toeplitz domain
        // dimension.
        debug_assert_eq!(self.toeplitz_domain.dimension(), f.len() - 1);

        self.toeplitz_domain.eval_prepared(
            &self.toeplitz_for_poly(&f),
            // The Toeplitz matrix is only evaluated on the powers up to max_poly_degree - 1,
            // since the H_j(X) polynomials have degree at most that
            &self.prepared_toeplitz_inputs[round],
        )
    }

    /// Compute the evaluation proofs for a KZG commitment of a polynomial `f`, committed to under
    /// `tau_powers`, on the FFT domain encapsulated by this [`FKDomain`].
    pub fn eval_proofs_at_roots_of_unity(&self, f: &[F], round: usize) -> Vec<T> {
        let h_term_commitments = self.compute_h_term_commitments(f, round);
        self.fft_domain.fft(&h_term_commitments)
    }

    pub fn eval_proofs_at_x_coords(&self, f: &[F], x_coords: &[F], round: usize) -> Vec<T> {
        let h_term_commitments = self.compute_h_term_commitments(f, round);
        multi_point_eval(&h_term_commitments, x_coords)
    }

    pub fn eval_proofs_at_x_coords_naive_multi_point_eval(
        &self,
        f: &[F],
        x_coords: &[F],
        round: usize,
    ) -> Vec<T> {
        let h_term_commitments = self.compute_h_term_commitments(f, round);

        multi_point_eval_naive(
            &h_term_commitments
                .into_iter()
                .map(T::MulBase::from)
                .collect::<Vec<T::MulBase>>(),
            x_coords,
        )
    }
}

impl<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> Serialize
    for FKDomain<F, T>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("FKDomain", 3)?;
        state.serialize_field(
            "toeplitz_domain_dimension",
            &self.toeplitz_domain.dimension(),
        )?;
        state.serialize_field("fft_domain_size", &self.fft_domain.size)?;
        state.serialize_field("prepared_toeplitz_inputs", &self.prepared_toeplitz_inputs)?;
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    ToeplitzDomainDimension,
    FftDomainSize,
    PreparedToeplitzInputs,
}

struct FKDomainVisitor<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> {
    _phantom: PhantomData<F>,
    _phantom2: PhantomData<T>,
}

impl<'de, F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> Visitor<'de>
    for FKDomainVisitor<F, T>
{
    type Value = FKDomain<F, T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct FKDomain")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<FKDomain<F, T>, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let toeplitz_domain_dimension = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let fft_domain_size = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let prepared_toeplitz_inputs = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?;
        Ok(FKDomain {
            toeplitz_domain: ToeplitzDomain::new(toeplitz_domain_dimension)
                .ok_or(de::Error::custom("Toeplitz domain initialization failed"))?,
            fft_domain: Radix2EvaluationDomain::new(fft_domain_size).ok_or(de::Error::custom(
                "Radix2EvaluationDomain initialization failed",
            ))?,
            prepared_toeplitz_inputs,
        })
    }

    fn visit_map<V>(self, mut map: V) -> Result<FKDomain<F, T>, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut toeplitz_domain_dimension: Option<usize> = None;
        let mut fft_domain_size: Option<usize> = None;
        let mut prepared_toeplitz_inputs: Option<Vec<PreparedInput<F, T>>> = None;
        while let Some(key) = map.next_key()? {
            match key {
                Field::ToeplitzDomainDimension => {
                    if toeplitz_domain_dimension.is_some() {
                        return Err(de::Error::duplicate_field("toeplitz_domain_dimension"));
                    }
                    toeplitz_domain_dimension = Some(map.next_value()?);
                },
                Field::FftDomainSize => {
                    if fft_domain_size.is_some() {
                        return Err(de::Error::duplicate_field("fft_domain_size"));
                    }
                    fft_domain_size = Some(map.next_value()?);
                },
                Field::PreparedToeplitzInputs => {
                    if prepared_toeplitz_inputs.is_some() {
                        return Err(de::Error::duplicate_field("prepared_toeplitz_inputs"));
                    }
                    prepared_toeplitz_inputs = Some(map.next_value()?);
                },
            }
        }
        let toeplitz_domain_dimension = toeplitz_domain_dimension
            .ok_or_else(|| de::Error::missing_field("toeplitz_domain_dimension"))?;
        let fft_domain_size =
            fft_domain_size.ok_or_else(|| de::Error::missing_field("fft_domain_size"))?;
        let prepared_toeplitz_inputs = prepared_toeplitz_inputs
            .ok_or_else(|| de::Error::missing_field("prepared_toeplitz_inputs"))?;
        Ok(FKDomain {
            toeplitz_domain: ToeplitzDomain::new(toeplitz_domain_dimension)
                .ok_or(de::Error::custom("Toeplitz domain initialization failed"))?,
            fft_domain: Radix2EvaluationDomain::new(fft_domain_size).ok_or(de::Error::custom(
                "Radix2EvaluationDomain initialization failed",
            ))?,
            prepared_toeplitz_inputs,
        })
    }
}

impl<'de, F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>
    Deserialize<'de> for FKDomain<F, T>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "toeplitz_domain_dimension",
            "fft_domain_size",
            "prepared_toeplitz_inputs",
        ];
        deserializer.deserialize_struct("FKDomain", FIELDS, FKDomainVisitor {
            _phantom: PhantomData,
            _phantom2: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FKDomain;
    use crate::group::{Fr, G1Affine, G1Projective, G2Affine, G2Projective, PairingSetting};
    use ark_ec::{pairing::Pairing, AffineRepr as _, PrimeGroup, ScalarMul as _, VariableBaseMSM};
    use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial, EvaluationDomain, Polynomial};
    use ark_std::{rand::thread_rng, One, UniformRand};

    #[test]
    fn test_serialize_deserialize() {
        for poly_degree_exp in 1..4 {
            let poly_degree = 2usize.pow(poly_degree_exp);
            let mut rng = thread_rng();

            let tau = Fr::rand(&mut rng);

            let mut tau_powers_fr = vec![Fr::one()];
            let mut cur = tau;
            for _ in 0..poly_degree {
                tau_powers_fr.push(cur);
                cur *= &tau;
            }

            let tau_powers_g1 = G1Projective::from(G1Affine::generator()).batch_mul(&tau_powers_fr);
            let tau_powers_g1_projective: Vec<Vec<G1Projective>> = vec![tau_powers_g1
                .iter()
                .map(|g| G1Projective::from(*g))
                .collect()];

            let fk_domain: FKDomain<Fr, G1Projective> =
                FKDomain::new(poly_degree, poly_degree, tau_powers_g1_projective).unwrap();

            let bytes = bcs::to_bytes(&fk_domain).unwrap();

            let fk_domain2: FKDomain<Fr, G1Projective> = bcs::from_bytes(&bytes).unwrap();

            assert_eq!(fk_domain, fk_domain2);

            let json = serde_json::to_string(&fk_domain).unwrap();

            let fk_domain2: FKDomain<Fr, G1Projective> = serde_json::from_str(&json).unwrap();

            assert_eq!(fk_domain, fk_domain2);
        }
    }

    #[test]
    fn compute_eval_proofs_at_roots_of_unity() {
        // TODO right now the only supported (max) polynomial degrees are powers of 2. Maybe I should change
        // that for better usability?
        for poly_degree_exp in 1..4 {
            let poly_degree = 2usize.pow(poly_degree_exp);
            let mut rng = thread_rng();

            let tau = Fr::rand(&mut rng);

            let mut tau_powers_fr = vec![Fr::one()];
            let mut cur = tau;
            for _ in 0..poly_degree {
                tau_powers_fr.push(cur);
                cur *= &tau;
            }

            let poly: DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
                (0..(poly_degree + 1)).map(|_| Fr::rand(&mut rng)).collect(),
            );

            let tau_powers_g1 = G1Projective::from(G1Affine::generator()).batch_mul(&tau_powers_fr);
            let tau_powers_g1_projective: Vec<Vec<G1Projective>> = vec![tau_powers_g1
                .iter()
                .map(|g| G1Projective::from(*g))
                .collect()];
            let tau_g2: G2Affine = (G2Affine::generator() * tau).into();

            let fk_domain =
                FKDomain::new(poly_degree, poly_degree, tau_powers_g1_projective).unwrap();
            let commitment: G1Affine = G1Projective::msm(&tau_powers_g1, &poly.coeffs)
                .unwrap()
                .into();

            let evaluation_proofs = fk_domain.eval_proofs_at_roots_of_unity(&poly.coeffs, 0);

            for (i, pf) in evaluation_proofs.iter().enumerate().take(poly_degree) {
                let lhs = PairingSetting::pairing(
                    G1Projective::from(commitment)
                        - (G1Projective::generator()
                            * poly.evaluate(&fk_domain.fft_domain.element(i))),
                    G2Projective::generator(),
                );
                let rhs = PairingSetting::pairing(
                    pf,
                    G2Projective::from(tau_g2)
                        - (G2Projective::generator() * fk_domain.fft_domain.element(i)),
                );
                assert_eq!(lhs, rhs);
            }
        }
    }
}
