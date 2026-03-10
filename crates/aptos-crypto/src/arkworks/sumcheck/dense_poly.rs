// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Dense multilinear polynomial with binding (from Jolt, no jolt dep).

use crate::arkworks::sumcheck::field::SumcheckField;

#[derive(Clone, Debug)]
pub struct DensePolynomial<F: SumcheckField> {
    pub num_vars: usize,
    pub len: usize,
    pub Z: Vec<F>,
}

#[derive(Clone, Copy, Debug)]
pub enum BindingOrder {
    LowToHigh,
    HighToLow,
}

impl<F: SumcheckField> DensePolynomial<F> {
    pub fn new(Z: Vec<F>) -> Self {
        assert!(
            Z.len().is_power_of_two(),
            "Dense multilinear polynomial must have power-of-2 length"
        );
        let num_vars = Z.len().trailing_zeros() as usize;
        let len = Z.len();
        Self { num_vars, len, Z }
    }

    pub fn get_num_vars(&self) -> usize {
        self.num_vars
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Bind one variable: fold Z into half length.
    /// - **HighToLow (MSB-first)**: pair Z[g] with Z[g+half] (first variable = MSB of index).
    /// - **LowToHigh (LSB-first)**: pair Z[2b] with Z[2b+1] (first variable = LSB of index).
    pub fn bind(&mut self, r: &F::Challenge, order: BindingOrder) {
        let r_f = F::challenge_to_field(r);
        let n = self.len / 2;
        match order {
            BindingOrder::HighToLow => {
                let (left, right) = self.Z.split_at_mut(n);
                for (a, b) in left.iter_mut().zip(right.iter()) {
                    *a += r_f * (*b - *a);
                }
                self.Z.truncate(n);
            },
            BindingOrder::LowToHigh => {
                let old_Z = std::mem::take(&mut self.Z);
                self.Z = (0..n)
                    .map(|i| {
                        let a = old_Z[2 * i];
                        let b = old_Z[2 * i + 1];
                        a + r_f * (b - a)
                    })
                    .collect();
            },
        }
        self.num_vars -= 1;
        self.len = n;
    }
}
