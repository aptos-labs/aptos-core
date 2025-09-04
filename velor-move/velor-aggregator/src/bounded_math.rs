// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, PartialEq, Eq)]
pub enum BoundedMathError {
    Overflow,
    Underflow,
}

pub type BoundedMathResult<T> = ::std::result::Result<T, BoundedMathError>;

pub fn ok_overflow<T>(value: BoundedMathResult<T>) -> BoundedMathResult<Option<T>> {
    match value {
        Ok(value) => Ok(Some(value)),
        Err(BoundedMathError::Overflow) => Ok(None),
        Err(BoundedMathError::Underflow) => Err(BoundedMathError::Underflow),
    }
}

pub fn ok_underflow<T>(value: BoundedMathResult<T>) -> BoundedMathResult<Option<T>> {
    match value {
        Ok(value) => Ok(Some(value)),
        Err(BoundedMathError::Overflow) => Err(BoundedMathError::Overflow),
        Err(BoundedMathError::Underflow) => Ok(None),
    }
}

fn negate_error<T>(result: BoundedMathResult<T>) -> BoundedMathResult<T> {
    result.map_err(|err| match err {
        BoundedMathError::Overflow => BoundedMathError::Underflow,
        BoundedMathError::Underflow => BoundedMathError::Overflow,
    })
}

// Unsigned operations operate on [0, max_value] range.
// Signed operations operate on [-max_value, max_value] range.
pub struct BoundedMath {
    max_value: u128,
}

impl BoundedMath {
    pub fn new(max_value: u128) -> Self {
        Self { max_value }
    }

    pub fn get_max_value(&self) -> u128 {
        self.max_value
    }

    pub fn unsigned_add(&self, base: u128, value: u128) -> BoundedMathResult<u128> {
        if self.max_value < base || value > (self.max_value - base) {
            Err(BoundedMathError::Overflow)
        } else {
            Ok(base + value)
        }
    }

    pub fn unsigned_subtract(&self, base: u128, value: u128) -> BoundedMathResult<u128> {
        if value > base {
            Err(BoundedMathError::Underflow)
        } else {
            Ok(base - value)
        }
    }

    pub fn unsigned_add_delta(&self, base: u128, delta: &SignedU128) -> BoundedMathResult<u128> {
        match delta {
            SignedU128::Positive(value) => self.unsigned_add(base, *value),
            SignedU128::Negative(value) => self.unsigned_subtract(base, *value),
        }
    }

    pub fn signed_add(
        &self,
        left: &SignedU128,
        right: &SignedU128,
    ) -> BoundedMathResult<SignedU128> {
        // Another useful macro, this time for merging deltas with different signs, such
        // as +A-B and -A+B. In these cases we have to check which of A or B is greater
        // and possibly flip a sign.
        macro_rules! update_different_sign {
            ($a:ident, $b:ident) => {
                if $a >= $b {
                    SignedU128::Positive(self.unsigned_subtract(*$a, *$b)?)
                } else {
                    SignedU128::Negative(self.unsigned_subtract(*$b, *$a)?)
                }
            };
        }

        Ok(match (left, right) {
            (SignedU128::Positive(v1), SignedU128::Positive(v2)) => {
                SignedU128::Positive(self.unsigned_add(*v1, *v2)?)
            },
            (SignedU128::Positive(v1), SignedU128::Negative(v2)) => update_different_sign!(v1, v2),
            (SignedU128::Negative(v1), SignedU128::Positive(v2)) => update_different_sign!(v2, v1),
            (SignedU128::Negative(v1), SignedU128::Negative(v2)) => {
                SignedU128::Negative(negate_error(self.unsigned_add(*v1, *v2))?)
            },
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SignedU128 {
    Positive(u128),
    Negative(u128),
}

impl PartialEq for SignedU128 {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Positive(v1), Self::Positive(v2)) | (Self::Negative(v1), Self::Negative(v2)) => {
                v1 == v2
            },
            (Self::Positive(v1), Self::Negative(v2)) | (Self::Negative(v1), Self::Positive(v2)) => {
                *v1 == 0 && *v2 == 0
            },
        }
    }
}

impl Eq for SignedU128 {}

impl SignedU128 {
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Positive(value) => *value == 0,
            Self::Negative(value) => *value == 0,
        }
    }

    pub fn delta(&self, positive: u128, negative: u128) -> Self {
        if positive >= negative {
            Self::Positive(positive - negative)
        } else {
            Self::Negative(negative - positive)
        }
    }

    pub fn minus(&self) -> Self {
        match self {
            Self::Positive(value) => Self::Negative(*value),
            Self::Negative(value) => Self::Positive(*value),
        }
    }

    pub fn abs(&self) -> u128 {
        match self {
            Self::Positive(value) | Self::Negative(value) => *value,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unsigned_add_delta() {
        let math = BoundedMath::new(100);
        assert_eq!(
            math.unsigned_add_delta(10, &SignedU128::Positive(5)),
            Ok(15)
        );
        assert_eq!(math.unsigned_add_delta(10, &SignedU128::Negative(5)), Ok(5));
        assert_eq!(
            math.unsigned_add_delta(10, &SignedU128::Positive(950)),
            Err(BoundedMathError::Overflow)
        );
        assert_eq!(
            math.unsigned_add_delta(10, &SignedU128::Negative(11)),
            Err(BoundedMathError::Underflow)
        );
    }

    #[test]
    fn test_delta_minus() {
        assert_eq!(SignedU128::Positive(10).minus(), SignedU128::Negative(10));
        assert_eq!(SignedU128::Negative(10).minus(), SignedU128::Positive(10));
    }

    #[test]
    fn test_signed_add() {
        let math = BoundedMath::new(100);
        assert_eq!(
            math.signed_add(&SignedU128::Positive(10), &SignedU128::Positive(5)),
            Ok(SignedU128::Positive(15))
        );
        assert_eq!(
            math.signed_add(&SignedU128::Positive(10), &SignedU128::Negative(5)),
            Ok(SignedU128::Positive(5))
        );
        assert_eq!(
            math.signed_add(&SignedU128::Negative(10), &SignedU128::Positive(5)),
            Ok(SignedU128::Negative(5))
        );
        assert_eq!(
            math.signed_add(&SignedU128::Negative(10), &SignedU128::Negative(5)),
            Ok(SignedU128::Negative(15))
        );
        assert_eq!(
            math.signed_add(&SignedU128::Positive(10), &SignedU128::Positive(90)),
            Ok(SignedU128::Positive(100))
        );
        assert_eq!(
            math.signed_add(&SignedU128::Positive(10), &SignedU128::Positive(91)),
            Err(BoundedMathError::Overflow)
        );
        assert_eq!(
            math.signed_add(&SignedU128::Negative(10), &SignedU128::Negative(90)),
            Ok(SignedU128::Negative(100))
        );
        assert_eq!(
            math.signed_add(&SignedU128::Negative(10), &SignedU128::Negative(91)),
            Err(BoundedMathError::Underflow)
        );
    }
}
