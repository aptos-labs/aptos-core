// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use either::Either;
use move_core_types::gas_algebra::{GasQuantity, UnitDiv};
use std::{
    marker::PhantomData,
    ops::{Add, Mul},
};

#[derive(Debug, Clone)]
pub enum Expression {
    //// Represent GasAdd
    Add {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    //// Represent GasMul
    Mul {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    //// Represent GasParam
    GasParam {
        name: String,
    },
    //// Represent GasValue
    GasValue {
        value: u64,
    },
}

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

    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit>;

    fn visit(&self, visitor: &mut impl GasExpressionVisitor);

    fn per<U>(self) -> GasPerUnit<Self, U>
    where
        Self: Sized,
    {
        GasPerUnit {
            inner: self,
            phantom: PhantomData,
        }
    }

    fn to_dynamic(&self) -> Expression {
        pub struct Visitor {
            //// Holds the AST
            pub node: Vec<Expression>,
        }

        impl GasExpressionVisitor for Visitor {
            fn add(&mut self) {
                let expr = Expression::Add {
                    left: (Box::new(self.node.pop().unwrap())),
                    right: (Box::new(self.node.pop().unwrap())),
                };
                self.node.push(expr);
            }

            fn mul(&mut self) {
                let expr = Expression::Mul {
                    left: (Box::new(self.node.pop().unwrap())),
                    right: (Box::new(self.node.pop().unwrap())),
                };
                self.node.push(expr);
            }

            fn gas_param<P>(&mut self) {
                let tn = std::any::type_name::<P>().split("::");
                let expr = Expression::GasParam {
                    name: (tn.last().unwrap().to_string()),
                };
                self.node.push(expr);
            }

            fn quantity<U>(&mut self, quantity: GasQuantity<U>) {
                let expr = Expression::GasValue {
                    value: (quantity.into()),
                };
                self.node.push(expr);
            }

            fn per<U>(&mut self) {
                return;
            }
        }

        let mut visitor = Visitor { node: Vec::new() };
        self.visit(&mut visitor);
        visitor
            .node
            .pop()
            .expect("there should be a root node in AST")
    }
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

    fn per<U>(&mut self);
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

#[derive(Debug, Clone)]
pub struct GasPerUnit<T, U> {
    pub inner: T,
    phantom: PhantomData<U>,
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

    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        (*self).evaluate(feature_version, env)
    }

    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        (*self).visit(visitor)
    }
}

impl<E, U> GasExpression<E> for GasQuantity<U> {
    type Unit = U;

    fn evaluate(&self, _feature_version: u64, _env: &E) -> GasQuantity<Self::Unit> {
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
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        self.left.evaluate(feature_version, env) + self.right.evaluate(feature_version, env)
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
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        self.left.evaluate(feature_version, env) * self.right.evaluate(feature_version, env)
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
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        match self {
            Either::Left(left) => left.evaluate(feature_version, env),
            Either::Right(right) => right.evaluate(feature_version, env),
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

impl<E, T, U1, U2> GasExpression<E> for GasPerUnit<T, U2>
where
    T: GasExpression<E, Unit = U1>,
{
    type Unit = UnitDiv<U1, U2>;

    #[inline]
    fn evaluate(&self, feature_version: u64, env: &E) -> GasQuantity<Self::Unit> {
        self.inner.evaluate(feature_version, env).per()
    }

    #[inline]
    fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
        self.inner.visit(visitor);
        visitor.per::<U2>();
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

impl<T, U, R> Add<R> for GasPerUnit<T, U> {
    type Output = GasAdd<Self, R>;

    fn add(self, rhs: R) -> Self::Output {
        GasAdd {
            left: self,
            right: rhs,
        }
    }
}

impl<T, U, R> Mul<R> for GasPerUnit<T, U> {
    type Output = GasMul<Self, R>;

    fn mul(self, rhs: R) -> Self::Output {
        GasMul {
            left: self,
            right: rhs,
        }
    }
}

// TODO: Add/Mul GasQuantity T
