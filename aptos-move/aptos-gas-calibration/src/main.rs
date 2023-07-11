// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_gas::GasAlgebra;
use aptos_gas_algebra::{
    Fee, FeePerGasUnit, GasExpression, GasExpressionVisitor, InternalGas, InternalGasUnit, Octa,
};
use aptos_gas_schedule::{StorageGasParameters, VMGasParameters};
use move_binary_format::errors::PartialVMResult;

enum Expression {
    Add {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Mul {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    GasParam {
        name: String,
    },
    GasValue {
        value: u64,
    },
}

struct CalibrationVisitor {
    node: Vec<Expression>,
}

impl GasExpressionVisitor for CalibrationVisitor {
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

    fn quantity<U>(&mut self, quantity: aptos_gas_algebra::GasQuantity<U>) {
        let expr = Expression::GasValue {
            value: (quantity.into()),
        };
        self.node.push(expr);
    }

    fn per<U>(&mut self) {
        return;
    }
}

// TODO: coefficients buffer
pub struct CalibrationAlgebra<A> {
    base: A,
}

impl<A: GasAlgebra> GasAlgebra for CalibrationAlgebra<A> {
    fn feature_version(&self) -> u64 {
        self.base.feature_version()
    }

    fn vm_gas_params(&self) -> &VMGasParameters {
        self.base.vm_gas_params()
    }

    fn storage_gas_params(&self) -> &StorageGasParameters {
        self.base.storage_gas_params()
    }

    fn balance_internal(&self) -> InternalGas {
        self.base.balance_internal()
    }

    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        //// Parse GasExpression into Expression AST
        abstract_amount.visit(&mut CalibrationVisitor { node: Vec::new() });

        //// Normalize (collect like terms)

        //// Put into buffer

        Ok(())
    }

    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        //// TODO
        abstract_amount.visit(&mut CalibrationVisitor { node: Vec::new() });
        Ok(())
    }

    fn charge_storage_fee(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        self.base
            .charge_storage_fee(abstract_amount, gas_unit_price)
    }

    fn execution_gas_used(&self) -> InternalGas {
        self.base.execution_gas_used()
    }

    fn io_gas_used(&self) -> InternalGas {
        self.base.io_gas_used()
    }

    fn storage_fee_used_in_gas_units(&self) -> InternalGas {
        self.base.storage_fee_used_in_gas_units()
    }

    fn storage_fee_used(&self) -> Fee {
        self.base.storage_fee_used()
    }
}

fn main() {
    println!("Hello, world!");
}
