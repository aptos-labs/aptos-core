use crate::{AptosGasParameters, Fee, FeePerGasUnit, Octa, StorageGasParameters};
use either::Either;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{GasQuantity, InternalGas, InternalGasUnit};
use std::ops::{Add, Mul};

pub trait GasExpressionVisitor {
    fn add(&mut self);

    fn mul(&mut self);

    fn gas_param<P>(&mut self);

    fn quantity<U>(&mut self, quantity: GasQuantity<U>);
}

/// Abstraction over a gas expression.
pub trait GasExpression {
    type Unit;

    fn materialize(
        &self,
        feature_version: u64,
        gas_params: &AptosGasParameters,
    ) -> GasQuantity<Self::Unit>;

    fn visit(&self, visitor: &mut impl GasExpressionVisitor);
}

pub trait GasAlgebra {
    fn feature_version(&self) -> u64;

    fn gas_params(&self) -> &AptosGasParameters;

    fn storage_gas_params(&self) -> &StorageGasParameters;

    fn balance_internal(&self) -> InternalGas;

    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<Unit = InternalGasUnit>,
    ) -> PartialVMResult<()>;

    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<Unit = InternalGasUnit>,
    ) -> PartialVMResult<()>;

    fn charge_storage_fee(
        &mut self,
        abstract_amount: impl GasExpression<Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()>;

    fn execution_gas_used(&self) -> InternalGas;

    fn io_gas_used(&self) -> InternalGas;

    fn storage_fee_used_in_gas_units(&self) -> InternalGas;

    fn storage_fee_used(&self) -> Fee;
}

pub struct GasAdd<L, R> {
    pub left: L,
    pub right: R,
}

pub struct GasMul<L, R> {
    pub left: L,
    pub right: R,
}

pub struct GasOptional<F, E> {
    pub predicate: F,
    pub exp: E,
}

impl<L, R, U> GasExpression for GasAdd<L, R>
where
    L: GasExpression<Unit = U>,
    R: GasExpression<Unit = U>,
{
    type Unit = U;

    #[inline]
    fn materialize(
        &self,
        feature_version: u64,
        gas_params: &AptosGasParameters,
    ) -> GasQuantity<Self::Unit> {
        self.left.materialize(feature_version, gas_params)
            + self.right.materialize(feature_version, gas_params)
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        self.left.visit(visitor);
        self.right.visit(visitor);
        visitor.add();
    }
}

impl<L, R, UL, UR, O> GasExpression for GasMul<L, R>
where
    L: GasExpression<Unit = UL>,
    R: GasExpression<Unit = UR>,
    GasQuantity<UL>: Mul<GasQuantity<UR>, Output = GasQuantity<O>>,
{
    type Unit = O;

    #[inline]
    fn materialize(
        &self,
        feature_version: u64,
        gas_params: &AptosGasParameters,
    ) -> GasQuantity<Self::Unit> {
        self.left.materialize(feature_version, gas_params)
            * self.right.materialize(feature_version, gas_params)
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        self.left.visit(visitor);
        self.right.visit(visitor);
        visitor.mul();
    }
}

impl<U> GasExpression for GasQuantity<U> {
    type Unit = U;

    #[inline]
    fn materialize(
        &self,
        _feature_version: u64,
        _gas_params: &AptosGasParameters,
    ) -> GasQuantity<Self::Unit> {
        *self
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        visitor.quantity(*self)
    }
}

impl<F, E> GasExpression for GasOptional<F, E>
where
    F: Fn() -> bool,
    E: GasExpression,
{
    type Unit = E::Unit;

    #[inline]
    fn materialize(
        &self,
        feature_version: u64,
        gas_params: &AptosGasParameters,
    ) -> GasQuantity<Self::Unit> {
        if (self.predicate)() {
            self.exp.materialize(feature_version, gas_params)
        } else {
            0.into()
        }
    }

    #[inline]
    fn visit(&self, _visitor: &mut impl GasExpressionVisitor) {}
}

impl<L, R, T> Add<T> for GasAdd<L, R>
where
    Self: GasExpression,
    T: GasExpression,
{
    type Output = GasAdd<Self, T>;

    fn add(self, rhs: T) -> Self::Output {
        GasAdd {
            left: self,
            right: rhs,
        }
    }
}

impl<L, R, T> Mul<T> for GasAdd<L, R>
where
    Self: GasExpression,
    T: GasExpression,
{
    type Output = GasMul<Self, T>;

    fn mul(self, rhs: T) -> Self::Output {
        GasMul {
            left: self,
            right: rhs,
        }
    }
}

impl<L, R, T> Add<T> for GasMul<L, R>
where
    Self: GasExpression,
    T: GasExpression,
{
    type Output = GasAdd<Self, T>;

    fn add(self, rhs: T) -> Self::Output {
        GasAdd {
            left: self,
            right: rhs,
        }
    }
}

impl<L, R, T> Mul<T> for GasMul<L, R>
where
    Self: GasExpression,
    T: GasExpression,
{
    type Output = GasMul<Self, T>;

    fn mul(self, rhs: T) -> Self::Output {
        GasMul {
            left: self,
            right: rhs,
        }
    }
}

impl<F, E, T> Add<T> for GasOptional<F, E>
where
    Self: GasExpression,
    T: GasExpression,
{
    type Output = GasAdd<Self, T>;

    fn add(self, rhs: T) -> Self::Output {
        GasAdd {
            left: self,
            right: rhs,
        }
    }
}

impl<F, E, T> Mul<T> for GasOptional<F, E>
where
    Self: GasExpression,
    T: GasExpression,
{
    type Output = GasMul<Self, T>;

    fn mul(self, rhs: T) -> Self::Output {
        GasMul {
            left: self,
            right: rhs,
        }
    }
}

impl<L, R, U> GasExpression for Either<L, R>
where
    L: GasExpression<Unit = U>,
    R: GasExpression<Unit = U>,
{
    type Unit = U;

    #[inline]
    fn materialize(
        &self,
        feature_version: u64,
        gas_params: &AptosGasParameters,
    ) -> GasQuantity<Self::Unit> {
        match self {
            Either::Left(left) => left.materialize(feature_version, gas_params),
            Either::Right(right) => right.materialize(feature_version, gas_params),
        }
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        match self {
            Either::Left(left) => left.visit(visitor),
            Either::Right(right) => right.visit(visitor),
        }
    }
}

pub mod gas_switches {
    use super::*;

    macro_rules! define_gas_switch {
        ($tn: ident < $($tp: ident),* $(,)? >) => {
            #[allow(unused)]
            pub enum $tn<$($tp),*> {
                $(
                    $tp($tp)
                ),*
            }

            impl<U, $($tp),*> GasExpression for $tn<$($tp),*>
            where
                $($tp: GasExpression<Unit = U>),*
            {
                type Unit = U;

                #[inline]
                fn materialize(
                    &self,
                    feature_version: u64,
                    gas_params: &AptosGasParameters,
                ) -> GasQuantity<Self::Unit> {
                    match self {
                        $(
                            $tn::$tp(e) => e.materialize(feature_version, gas_params)
                        ),*
                    }
                }

                #[inline]
                fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
                    match self {
                        $(
                            $tn::$tp(e) => e.visit(visitor)
                        ),*
                    }
                }
            }
        };
    }

    define_gas_switch!(GasSwitch2<E1, E2>);
    define_gas_switch!(GasSwitch3<E1, E2, E3>);
    define_gas_switch!(GasSwitch4<E1, E2, E3, E4>);
    define_gas_switch!(GasSwitch5<E1, E2, E3, E4, E5>);
    define_gas_switch!(GasSwitch6<E1, E2, E3, E4, E5, E6>);
    define_gas_switch!(GasSwitch7<E1, E2, E3, E4, E5, E6, E7>);
    define_gas_switch!(GasSwitch8<E1, E2, E3, E4, E5, E6, E7, E8>);
}

impl<T> GasExpression for &T
where
    T: GasExpression,
{
    type Unit = T::Unit;

    fn materialize(
        &self,
        feature_version: u64,
        gas_params: &AptosGasParameters,
    ) -> GasQuantity<Self::Unit> {
        (*self).materialize(feature_version, gas_params)
    }

    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        (*self).visit(visitor)
    }
}
