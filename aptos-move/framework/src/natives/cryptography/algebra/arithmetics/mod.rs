// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod add;
pub mod div;
pub mod double;
pub mod inv;
pub mod mul;
pub mod neg;
pub mod scalar_mul;
pub mod sqr;
pub mod sub;

#[macro_export]
macro_rules! ark_binary_op_internal {
    ($context:expr_2021, $args:ident, $ark_typ:ty, $ark_func:ident, $gas:expr_2021) => {{
        let handle_2 = aptos_native_interface::safely_pop_arg!($args, u64) as usize;
        let handle_1 = aptos_native_interface::safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $ark_typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $ark_typ, element_2_ptr, element_2);
        $context.charge($gas)?;
        let new_element = element_1.$ark_func(element_2);
        let new_handle = store_element!($context, new_element)?;
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

#[macro_export]
macro_rules! ark_unary_op_internal {
    ($context:expr_2021, $args:ident, $ark_typ:ty, $ark_func:ident, $gas:expr_2021) => {{
        let handle = aptos_native_interface::safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $ark_typ, element_ptr, element);
        $context.charge($gas)?;
        let new_element = element.$ark_func();
        let new_handle = store_element!($context, new_element)?;
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}
