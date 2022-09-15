// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err};
use serde::de::Error;
use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

pub type AptosCoin = FixedDecimalCoin<AptosCoinInfo>;

/// A marker trait to keep track of decimals and naming, similar to the corresponding move type
pub trait FixedDecimalCoinInfo {
    /// Number of decimal points
    const NUM_DECIMALS: u8;
    /// Symbol of coin
    const SYMBOL: &'static str;
    /// Subunit name
    const SUBUNIT: &'static str;
    /// Offset based on decimals for a full coin
    const COIN_OFFSET: u64 = u64::pow(10, Self::NUM_DECIMALS as u32);
}

/// A fixed point representation for APT
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct AptosCoinInfo {}

impl FixedDecimalCoinInfo for AptosCoinInfo {
    const NUM_DECIMALS: u8 = 8;
    const SYMBOL: &'static str = "APT";
    const SUBUNIT: &'static str = "Octa";
}

/// A fixed decimal coin for parsing
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct FixedDecimalCoin<T: FixedDecimalCoinInfo + 'static> {
    amount: u64,
    _marker: std::marker::PhantomData<&'static T>,
}

impl<T: FixedDecimalCoinInfo> FixedDecimalCoin<T> {
    pub fn new(amount: u64) -> Self {
        FixedDecimalCoin {
            amount,
            _marker: Default::default(),
        }
    }
    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl<T: FixedDecimalCoinInfo> From<&FixedDecimalCoin<T>> for u64 {
    fn from(coin: &FixedDecimalCoin<T>) -> Self {
        coin.amount()
    }
}

impl<T: FixedDecimalCoinInfo> From<u64> for FixedDecimalCoin<T> {
    fn from(amount: u64) -> Self {
        FixedDecimalCoin::new(amount)
    }
}

impl<T: FixedDecimalCoinInfo> serde::Serialize for FixedDecimalCoin<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        S::serialize_str(serializer, self.to_string().as_str())
    }
}

impl<'de, T: FixedDecimalCoinInfo> serde::Deserialize<'de> for FixedDecimalCoin<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str = <String>::deserialize(deserializer)?;
        FixedDecimalCoin::from_str(&str)
            .map_err(|err| D::Error::custom(format!("Failed to parse into {} {}", T::SYMBOL, err)))
    }
}

impl<T: FixedDecimalCoinInfo> Add for FixedDecimalCoin<T> {
    type Output = FixedDecimalCoin<T>;

    fn add(self, rhs: Self) -> Self::Output {
        FixedDecimalCoin::new(self.amount + rhs.amount)
    }
}

impl<T: FixedDecimalCoinInfo> Sub for FixedDecimalCoin<T> {
    type Output = FixedDecimalCoin<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        FixedDecimalCoin::new(self.amount - rhs.amount)
    }
}

impl<T: FixedDecimalCoinInfo> Mul for FixedDecimalCoin<T> {
    type Output = FixedDecimalCoin<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        FixedDecimalCoin::new(self.amount * rhs.amount)
    }
}

impl<T: FixedDecimalCoinInfo> Div for FixedDecimalCoin<T> {
    type Output = FixedDecimalCoin<T>;

    fn div(self, rhs: Self) -> Self::Output {
        FixedDecimalCoin::new(self.amount / rhs.amount)
    }
}

impl<T: FixedDecimalCoinInfo> std::fmt::Display for FixedDecimalCoin<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let upper = self.amount / T::COIN_OFFSET;
        let lower = self.amount - (upper * T::COIN_OFFSET);

        write!(f, "{}.{}", upper, lower)
    }
}

impl<T: FixedDecimalCoinInfo> std::fmt::Debug for FixedDecimalCoin<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl<T: FixedDecimalCoinInfo> FromStr for FixedDecimalCoin<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split the parts on the decimal
        let pieces: Vec<&str> = s.trim().split('.').collect();

        let amount = match (pieces.len(), pieces.first(), pieces.get(1)) {
            // If there is no decimal, it's a full coin
            (1, Some(coin), None) => {
                if let Some(amount) = u64::from_str(coin)
                    .map_err(|err| format_err!("Unable to parse {}: {}", T::SYMBOL, err))?
                    .checked_mul(T::COIN_OFFSET)
                {
                    amount
                } else {
                    bail!(
                        "Unable to parse {}: Number is too large to handle {} decimal points {}",
                        T::SYMBOL,
                        T::NUM_DECIMALS,
                        s
                    );
                }
            }
            // If there's a decimal, then there are subunits
            (2, Some(coin), Some(subunit)) => {
                let coin = if !coin.is_empty() {
                    u64::from_str(coin)
                        .map_err(|err| format_err!("Unable to parse {}: {}", T::SYMBOL, err))?
                        * T::COIN_OFFSET
                } else {
                    0
                };

                let subunit = if subunit.len() > T::NUM_DECIMALS as usize {
                    bail!(
                        "Unable to parse {}: Too many decimal points, expected {} or less, but got {}: {}",
                        T::SYMBOL,
                        T::NUM_DECIMALS,
                        subunit.len(),
                        s
                    )
                } else if !subunit.is_empty() {
                    // Fill in the missing zeros to the right of the subunit
                    let offset: u64 = u64::pow(10, T::NUM_DECIMALS as u32 - subunit.len() as u32);

                    if let Some(amount) = u64::from_str(subunit)
                        .map_err(|err| format_err!("Unable to parse {}: {}", T::SUBUNIT, err))?
                        .checked_mul(offset)
                    {
                        amount
                    } else {
                        bail!(
                            "Unable to parse {}: Failed to parse {} decimal: {}",
                            T::SYMBOL,
                            T::SUBUNIT,
                            s
                        )
                    }
                } else {
                    0
                };

                coin + subunit
            }
            _ => bail!(
                "Unable to parse {}: More than one decimal point in the input {}",
                T::SYMBOL,
                s
            ),
        };

        Ok(FixedDecimalCoin::new(amount))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fixed_point() {
        let tests = [
            ("1", AptosCoinInfo::COIN_OFFSET),
            ("0.00000001", 1),
            ("0.1", AptosCoinInfo::COIN_OFFSET / 10),
            ("10000", 10000 * AptosCoinInfo::COIN_OFFSET),
            (
                "10000.01",
                10000 * AptosCoinInfo::COIN_OFFSET + AptosCoinInfo::COIN_OFFSET / 100,
            ),
            (".1", AptosCoinInfo::COIN_OFFSET / 10),
            ("1.0", AptosCoinInfo::COIN_OFFSET),
            ("1.", AptosCoinInfo::COIN_OFFSET),
        ];

        for (str, expected) in tests {
            let result = AptosCoin::from_str(str).unwrap().amount();
            assert_eq!(
                result, expected,
                "Testcase: {} expected {} got {}",
                str, expected, result
            );
        }

        let bad_tests = ["1.1.", "10000000000000000", "0.000000001", "not_a_number"];
        for str in bad_tests {
            AptosCoin::from_str(str).expect_err(str);
        }

        let yaml = "1000.00000001";
        assert_eq!(
            1000_0000_0001,
            serde_yaml::from_str::<AptosCoin>(yaml).unwrap().amount()
        );
    }
}
