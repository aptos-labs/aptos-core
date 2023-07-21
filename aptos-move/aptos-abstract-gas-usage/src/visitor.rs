// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::{DynamicExpression, GasExpressionVisitor};

/// Visitor to traverse the Reverse Polish Notation
pub struct CalibrationVisitor {
    //// Holds the AST
    pub node: Vec<DynamicExpression>,
}

/// CalibrationVisitor implementation
impl GasExpressionVisitor for CalibrationVisitor {
    fn add(&mut self) {
        let expr = DynamicExpression::Add {
            left: (Box::new(self.node.pop().unwrap())),
            right: (Box::new(self.node.pop().unwrap())),
        };
        self.node.push(expr);
    }

    fn mul(&mut self) {
        let expr = DynamicExpression::Mul {
            left: (Box::new(self.node.pop().unwrap())),
            right: (Box::new(self.node.pop().unwrap())),
        };
        self.node.push(expr);
    }

    fn gas_param<P>(&mut self) {
        let tn = std::any::type_name::<P>().split("::");
        let expr = DynamicExpression::GasParam {
            name: (tn.last().unwrap().to_string()),
        };
        self.node.push(expr);
    }

    fn quantity<U>(&mut self, quantity: aptos_gas_algebra::GasQuantity<U>) {
        let expr = DynamicExpression::GasValue {
            value: (quantity.into()),
        };
        self.node.push(expr);
    }

    fn per<U>(&mut self) {
        return;
    }
}
