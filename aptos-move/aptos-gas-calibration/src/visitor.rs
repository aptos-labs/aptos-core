// Copyright © Aptos Foundation

use crate::types::Expression;
use aptos_gas_algebra::GasExpressionVisitor;

pub struct CalibrationVisitor {
    pub node: Vec<Expression>,
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
