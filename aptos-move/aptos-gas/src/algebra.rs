use serde::{Deserialize, Serialize};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::convert::From;
use std::fmt::{self, Debug, Display};
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

/***************************************************************************************************
 * Units of Measurement
 *
 **************************************************************************************************/
/// Unit of (external) gas.
pub enum GasUnit {}

/// Unit of internal gas.
pub enum InternalGasUnit {}

/// Unit for counting bytes.
pub enum Byte {}

/// Unit of gas currency. 1 octa = 10^-8 Aptos coins.
pub enum Octa {}

// Unit of abstract memory usage in the Move VM.
pub enum AbstractMemoryUnit {}

/// A drived unit resulted from the division of two given units.
/// This is used to permit type-safe multiplications.
///
/// A real life example: 3 m/s * 5 s = 15 m
pub struct UnitDiv<U1, U2> {
    phantom: PhantomData<(U1, U2)>,
}

/***************************************************************************************************
 * Gas Quantities
 *
 **************************************************************************************************/
/// An opqaue representation of a certain quantity, with the unit being encoded in the type.
/// This type implements checked addition and subtraction, and only permits type-safe multiplication.
#[derive(Serialize, Deserialize)]
pub struct GasQuantity<U> {
    val: u64,
    phantom: PhantomData<U>,
}

pub type Gas = GasQuantity<GasUnit>;

pub type InternalGas = GasQuantity<InternalGasUnit>;

pub type NumBytes = GasQuantity<Byte>;

pub type InternalGasPerByte = GasQuantity<UnitDiv<InternalGasUnit, Byte>>;

pub type InternalGasPerAbstractMemoryUnit =
    GasQuantity<UnitDiv<InternalGasUnit, AbstractMemoryUnit>>;

pub type AbstractMemorySize = GasQuantity<AbstractMemoryUnit>;

pub type GasScalingFactor = GasQuantity<UnitDiv<InternalGasUnit, GasUnit>>;

pub type Fee = GasQuantity<Octa>;

pub type FeePerGasUnit = GasQuantity<UnitDiv<Octa, GasUnit>>;

/***************************************************************************************************
 * Constructors
 *
 **************************************************************************************************/
impl<U> GasQuantity<U> {
    pub fn new(val: u64) -> Self {
        Self {
            val,
            phantom: PhantomData,
        }
    }

    pub fn zero() -> Self {
        Self::new(0)
    }
}

/***************************************************************************************************
 * Conversion
 *
 **************************************************************************************************/
impl<U> From<u64> for GasQuantity<U> {
    fn from(val: u64) -> Self {
        Self::new(val)
    }
}

// TODO(Gas): This allows the gas value to escape the monad, which weakens the type-level
//            protection it provides. It's currently needed for practical reasons but
//            we should look for ways to get rid of it.
impl<U> From<GasQuantity<U>> for u64 {
    fn from(gas: GasQuantity<U>) -> Self {
        gas.val
    }
}

/***************************************************************************************************
 * Clone & Copy
 *
 **************************************************************************************************/
impl<U> Clone for GasQuantity<U> {
    fn clone(&self) -> Self {
        Self::new(self.val)
    }
}

impl<U> Copy for GasQuantity<U> {}

/***************************************************************************************************
 * Display & Debug
 *
 **************************************************************************************************/
impl<U> Display for GasQuantity<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.val)
    }
}

impl<U> Debug for GasQuantity<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.val, std::any::type_name::<U>())
    }
}

/***************************************************************************************************
 * Comparison
 *
 **************************************************************************************************/
impl<U> GasQuantity<U> {
    fn cmp_impl(&self, other: &Self) -> Ordering {
        self.val.cmp(&other.val)
    }
}

impl<U> PartialEq for GasQuantity<U> {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp_impl(other), Ordering::Equal)
    }
}

impl<U> Eq for GasQuantity<U> {}

impl<U> PartialOrd for GasQuantity<U> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp_impl(other))
    }
}

impl<U> Ord for GasQuantity<U> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_impl(other)
    }
}

/***************************************************************************************************
 * Addition & Subtraction
 *
 **************************************************************************************************/
impl<U> Add<GasQuantity<U>> for GasQuantity<U> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.val
                .checked_add(rhs.val)
                .unwrap_or_else(|| panic!("overflow when calculating ({:?}) + ({:?})", self, rhs)),
        )
    }
}

impl<U> AddAssign<GasQuantity<U>> for GasQuantity<U> {
    fn add_assign(&mut self, rhs: GasQuantity<U>) {
        *self = *self + rhs
    }
}

impl<U> Sub<GasQuantity<U>> for GasQuantity<U> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(
            self.val
                .checked_sub(rhs.val)
                .unwrap_or_else(|| panic!("underflow when calculating ({:?}) - ({:?})", self, rhs)),
        )
    }
}

impl<U> SubAssign<GasQuantity<U>> for GasQuantity<U> {
    fn sub_assign(&mut self, rhs: GasQuantity<U>) {
        *self = *self - rhs
    }
}

/***************************************************************************************************
 * Multiplication
 *
 **************************************************************************************************/
fn mul_impl<U1, U2>(x: GasQuantity<U2>, y: GasQuantity<UnitDiv<U1, U2>>) -> GasQuantity<U1> {
    GasQuantity::new(
        x.val
            .checked_mul(y.val)
            .unwrap_or_else(|| panic!("overflow when calculating ({:?}) * ({:?})", x, y)),
    )
}

impl<U1, U2> Mul<GasQuantity<UnitDiv<U1, U2>>> for GasQuantity<U2> {
    type Output = GasQuantity<U1>;

    fn mul(self, rhs: GasQuantity<UnitDiv<U1, U2>>) -> Self::Output {
        mul_impl(self, rhs)
    }
}

impl<U1, U2> Mul<GasQuantity<U2>> for GasQuantity<UnitDiv<U1, U2>> {
    type Output = GasQuantity<U1>;

    fn mul(self, rhs: GasQuantity<U2>) -> Self::Output {
        mul_impl(rhs, self)
    }
}
