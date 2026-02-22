//! Defines the data structures used by the `MLSumcheck` protocol.

use ark_ff::Field;
use ark_poly::{DenseMultilinearExtension, Polynomial};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{cmp::max, rc::Rc, vec::Vec};
use hashbrown::HashMap;
/// Stores a list of products of `DenseMultilinearExtension` that is meant to be added together.
///
/// The polynomial is represented by a list of products of polynomials along with its coefficient that is meant to be added together.
///
/// This data structure of the polynomial is a list of list of `(coefficient, DenseMultilinearExtension)`.
/// * Number of products n = `self.products.len()`,
/// * Number of multiplicands of ith product m_i = `self.products[i].1.len()`,
/// * Coefficient of ith product c_i = `self.products[i].0`
///
/// The resulting polynomial is
///
/// $$\sum_{i=0}^{n}C_i\cdot\prod_{j=0}^{m_i}P_{ij}$$
///
/// The result polynomial is used as the prover key.
#[derive(Clone)]
pub struct ListOfProductsOfPolynomials<F: Field> {
    /// max number of multiplicands in each product
    pub max_multiplicands: usize,
    /// number of variables of the polynomial
    pub num_variables: usize,
    /// list of reference to products (as usize) of multilinear extension
    pub products: Vec<(F, Vec<usize>)>,
    /// Stores multilinear extensions in which product multiplicand can refer to.
    pub flattened_ml_extensions: Vec<Rc<DenseMultilinearExtension<F>>>,
    raw_pointers_lookup_table: HashMap<*const DenseMultilinearExtension<F>, usize>,
}

impl<F: Field> ListOfProductsOfPolynomials<F> {
    /// Extract the max number of multiplicands and number of variables of the list of products.
    pub fn info(&self) -> PolynomialInfo {
        PolynomialInfo {
            max_multiplicands: self.max_multiplicands,
            num_variables: self.num_variables,
        }
    }
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
/// Stores the number of variables and max number of multiplicands of the added polynomial used by the prover.
/// This data structures will is used as the verifier key.
pub struct PolynomialInfo {
    /// max number of multiplicands in each product
    pub max_multiplicands: usize,
    /// number of variables of the polynomial
    pub num_variables: usize,
}

impl<F: Field> ListOfProductsOfPolynomials<F> {
    /// Returns an empty polynomial
    pub fn new(num_variables: usize) -> Self {
        ListOfProductsOfPolynomials {
            max_multiplicands: 0,
            num_variables,
            products: Vec::new(),
            flattened_ml_extensions: Vec::new(),
            raw_pointers_lookup_table: HashMap::new(),
        }
    }

    /// Add a list of multilinear extensions that is meant to be multiplied together.
    /// The resulting polynomial will be multiplied by the scalar `coefficient`.
    pub fn add_product(
        &mut self,
        product: impl IntoIterator<Item = Rc<DenseMultilinearExtension<F>>>,
        coefficient: F,
    ) {
        let product: Vec<Rc<DenseMultilinearExtension<F>>> = product.into_iter().collect();
        let mut indexed_product = Vec::with_capacity(product.len());
        assert!(!product.is_empty());
        self.max_multiplicands = max(self.max_multiplicands, product.len());
        for m in product {
            assert_eq!(
                m.num_vars, self.num_variables,
                "product has a multiplicand with wrong number of variables"
            );
            let m_ptr: *const DenseMultilinearExtension<F> = Rc::as_ptr(&m);
            if let Some(index) = self.raw_pointers_lookup_table.get(&m_ptr) {
                indexed_product.push(*index)
            } else {
                let curr_index = self.flattened_ml_extensions.len();
                self.flattened_ml_extensions.push(m.clone());
                self.raw_pointers_lookup_table.insert(m_ptr, curr_index);
                indexed_product.push(curr_index);
            }
        }
        self.products.push((coefficient, indexed_product));
    }

    /// Evaluate the polynomial at point `point`
    pub fn evaluate(&self, point: &[F]) -> F {
        self.products
            .iter()
            .map(|(c, p)| {
                *c * p
                    .iter()
                    .map(|&i| self.flattened_ml_extensions[i].evaluate(&point.to_vec()))
                    .product::<F>()
            })
            .sum()
    }
}

/// Represents a polynomial of the form
/// [L(X) + Σᵢ cᵢ·Pᵢ(1-Pᵢ)] · eq_t(X) · (1 - eq_{0,...,0}(X)) + α · g(X)
/// where L is an optional linear term (e.g. f - Σ 2^{j-1} f_j), eq_{0,...,0}(x) = ∏ᵢ(1-xᵢ), and
/// g(X) = g₁(X₁) + ... + gₙ(Xₙ) with each gᵢ degree-4 univariate.
pub struct BinaryConstraintPolynomial<F: Field> {
    /// Optional linear term (e.g. f - sum 2^{j-1} f_j); multiplied by eq_t · (1 - eq_zero)
    pub linear_term: Option<DenseMultilinearExtension<F>>,
    /// List of (coefficient, polynomial) pairs for the binary constraints
    pub constraints: Vec<(F, DenseMultilinearExtension<F>)>,
    /// The point t for eq_t
    pub eq_point: Vec<F>,
    /// Coefficient α for the g term
    pub alpha: F,
    /// Random univariate polynomials g₁, ..., gₙ
    /// Each is represented as [r₀, r₁, r₂, r₃, r₄] (coefficients for degree-4 polynomial)
    pub g_polys: Vec<Vec<F>>,
    /// Number of variables
    pub num_variables: usize,
}

impl<F: Field> BinaryConstraintPolynomial<F> {
    /// Create new polynomial with eq_t evaluation point and random g polynomials
    pub fn new(num_variables: usize, eq_point: Vec<F>, alpha: F, g_polys: Vec<Vec<F>>) -> Self {
        if eq_point.len() != num_variables {
            panic!("eq_point must have same dimension as num_variables");
        }
        if g_polys.len() != num_variables {
            panic!("Must have one g polynomial per variable");
        }
        for (i, g) in g_polys.iter().enumerate() {
            if g.len() != 5 {
                panic!("g_poly[{}] must have 5 coefficients (degree 4)", i);
            }
        }

        Self {
            linear_term: None,
            constraints: Vec::new(),
            eq_point,
            alpha,
            g_polys,
            num_variables,
        }
    }

    /// Set the optional linear term L (e.g. f - sum 2^{j-1} f_j).
    pub fn set_linear_term(&mut self, l: DenseMultilinearExtension<F>) {
        assert_eq!(l.num_vars, self.num_variables);
        self.linear_term = Some(l);
    }

    /// Add a binary constraint: c · P(1-P)
    pub fn add_constraint(&mut self, coefficient: F, polynomial: DenseMultilinearExtension<F>) {
        if polynomial.num_vars != self.num_variables {
            panic!("Polynomial has wrong number of variables");
        }
        self.constraints.push((coefficient, polynomial));
    }

    /// Evaluate gᵢ(x) where gᵢ(X) = r₀ + r₁X + r₂X² + r₃X³ + r₄X⁴
    fn eval_g_i(&self, var_index: usize, x: F) -> F {
        let coeffs = &self.g_polys[var_index];
        let mut result = coeffs[0];
        let mut x_pow = x;
        for i in 1..5 {
            result += coeffs[i] * x_pow;
            x_pow *= x;
        }
        result
    }

    /// Evaluate g(x) = g₁(x₁) + ... + gₙ(xₙ)
    fn eval_g(&self, point: &[F]) -> F {
        let mut sum = F::zero();
        for i in 0..self.num_variables {
            sum += self.eval_g_i(i, point[i]);
        }
        sum
    }

    /// Evaluate eq_t(x) = ∏ᵢ [tᵢ·xᵢ + (1-tᵢ)·(1-xᵢ)]
    fn eval_eq(&self, point: &[F]) -> F {
        let mut result = F::one();
        for i in 0..self.num_variables {
            let ti = self.eq_point[i];
            let xi = point[i];
            result *= (F::one() - ti) + xi * (ti + ti - F::one());
        }
        result
    }

    /// Evaluate eq_{0,...,0}(x) = ∏ᵢ (1 - xᵢ) (equals 1 iff x = (0,...,0))
    fn eval_eq_all_zeros(&self, point: &[F]) -> F {
        let mut result = F::one();
        for &xi in point {
            result *= F::one() - xi;
        }
        result
    }

    /// Evaluate the full polynomial at a point
    pub fn evaluate(&self, point: &[F]) -> F {
        // Linear term L(x)
        let linear_val = self
            .linear_term
            .as_ref()
            .map(|l| l.evaluate(&point.to_vec()))
            .unwrap_or(F::zero());

        // Compute Σᵢ cᵢ·Pᵢ(x)·(1-Pᵢ(x))
        let mut binary_sum = linear_val;
        for (coeff, poly) in &self.constraints {
            let p_val = poly.evaluate(&point.to_vec());
            binary_sum += *coeff * p_val * (F::one() - p_val);
        }

        // Compute eq_t(x)
        let eq_t = self.eval_eq(point);

        // Compute eq_{0,...,0}(x) = ∏ᵢ(1-xᵢ)
        let eq_zero = self.eval_eq_all_zeros(point);

        // Compute g(x)
        let g_val = self.eval_g(point);

        // Return [L + Σᵢ cᵢ·Pᵢ·(1-Pᵢ)] · eq_t · (1 - eq_{0,...,0}) + α · g
        binary_sum * eq_t * (F::one() - eq_zero) + self.alpha * g_val
    }

    /// Get polynomial info for verifier
    pub fn info(&self) -> PolynomialInfo {
        PolynomialInfo {
            num_variables: self.num_variables,
            max_multiplicands: 2,
        }
    }

    /// Create new polynomial without g polynomials (α = 0, g = 0)
    /// This is a convenience method for tests that don't need the g term
    pub fn new_without_g(num_variables: usize, eq_point: Vec<F>) -> Self {
        let alpha = F::zero();
        let g_polys = vec![vec![F::zero(); 5]; num_variables];
        Self::new(num_variables, eq_point, alpha, g_polys)
    }
}
