// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use either::Either;
use move_core_types::gas_algebra::GasQuantity;
use std::ops::{Add, Mul};

/***************************************************************************************************
 * Gas Expression & Visitor
 *
 **************************************************************************************************/
/// Trait representing an abstract view over an expression that is used to represent some
/// gas amount.
///
/// It carries a type parameter `E`, indicating an environment in which the expression can be
/// evaluated/materialized.
pub trait GasExpression<E> {
    type Unit;

    fn materialize(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit>;

    fn visit(&self, visitor: &mut impl GasExpressionVisitor);
}

/// An interface for performing post-order traversal of the tree structure of a gas expression.
///
/// Alternatively, one could think that the callbacks are invoked following the Reverse Polish
/// notation of the expression.
///
/// Here are a few examples:
/// - `1 + 2`
///   - `quantity(1)`
///   - `quantity(2)`
///   - `add()`
/// - `A + B * 50`
///   - `gas_param<A>()`
///   - `gas_param<B>()`
///   - `quantity(50)`
///   - `mul()`
///   - `add()`
pub trait GasExpressionVisitor {
    fn add(&mut self);

    fn mul(&mut self);

    fn gas_param<P>(&mut self);

    fn quantity<U>(&mut self, quantity: GasQuantity<U>);
}

/***************************************************************************************************
 * Built-in Gas Expressions
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasAdd<L, R> {
    pub left: L,
    pub right: R,
}

#[derive(Debug, Clone)]
pub struct GasMul<L, R> {
    pub left: L,
    pub right: R,
}

/***************************************************************************************************
 * Gas Expression Impl
 *
 **************************************************************************************************/
impl<E, T> GasExpression<E> for &T
where
    T: GasExpression<E>,
{
    type Unit = T::Unit;

    fn materialize(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        (*self).materialize(feature_version, env)
    }

    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        (*self).visit(visitor)
    }
}

impl<E, U> GasExpression<E> for GasQuantity<U> {
    type Unit = U;

    fn materialize(&self, _feature_version: u64, _env: &E) -> GasQuantity<Self::Unit> {
        *self
    }

    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        visitor.quantity(*self)
    }
}

impl<E, L, R, U> GasExpression<E> for GasAdd<L, R>
where
    L: GasExpression<E, Unit = U>,
    R: GasExpression<E, Unit = U>,
{
    type Unit = U;

    #[inline]
    fn materialize(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        self.left.materialize(feature_version, env) + self.right.materialize(feature_version, env)
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        self.left.visit(visitor);
        self.right.visit(visitor);
        visitor.add();
    }
}

impl<E, L, R, UL, UR, O> GasExpression<E> for GasMul<L, R>
where
    L: GasExpression<E, Unit = UL>,
    R: GasExpression<E, Unit = UR>,
    GasQuantity<UL>: Mul<GasQuantity<UR>, Output = GasQuantity<O>>,
{
    type Unit = O;

    #[inline]
    fn materialize(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        self.left.materialize(feature_version, env) * self.right.materialize(feature_version, env)
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        self.left.visit(visitor);
        self.right.visit(visitor);
        visitor.mul();
    }
}

impl<E, L, R, U> GasExpression<E> for Either<L, R>
where
    L: GasExpression<E, Unit = U>,
    R: GasExpression<E, Unit = U>,
{
    type Unit = U;

    #[inline]
    fn materialize(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        match self {
            Either::Left(left) => left.materialize(feature_version, env),
            Either::Right(right) => right.materialize(feature_version, env),
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

/***************************************************************************************************
 * Arithmetic Operations
 *
 **************************************************************************************************/
impl<L, R, T> Add<T> for GasAdd<L, R> {
    type Output = GasAdd<Self, T>;

    fn add(self, rhs: T) -> Self::Output {
        GasAdd {
            left: self,
            right: rhs,
        }
    }
}

impl<L, R, T> Mul<T> for GasAdd<L, R> {
    type Output = GasMul<Self, T>;

    fn mul(self, rhs: T) -> Self::Output {
        GasMul {
            left: self,
            right: rhs,
        }
    }
}

impl<L, R, T> Add<T> for GasMul<L, R> {
    type Output = GasAdd<Self, T>;

    fn add(self, rhs: T) -> Self::Output {
        GasAdd {
            left: self,
            right: rhs,
        }
    }
}

impl<L, R, T> Mul<T> for GasMul<L, R> {
    type Output = GasMul<Self, T>;

    fn mul(self, rhs: T) -> Self::Output {
        GasMul {
            left: self,
            right: rhs,
        }
    }
}

// TODO: Add/Mul GasQuantity T
