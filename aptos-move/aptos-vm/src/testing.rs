// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::VMResult;

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum InjectedError {
    EndOfRunEpilogue,
}

pub(crate) fn maybe_raise_injected_error(_error_type: InjectedError) -> VMResult<()> {
    #[cfg(feature = "testing")]
    {
        testing_only::maybe_raise_injected_error(_error_type)
    }

    #[cfg(not(feature = "testing"))]
    Ok(())
}

#[cfg(feature = "testing")]
pub mod testing_only {
    use super::InjectedError;
    use move_binary_format::errors::{Location, PartialVMError, VMResult};
    use move_core_types::vm_status::StatusCode;
    use std::{cell::RefCell, collections::HashSet};

    thread_local! {
        static INJECTED_ERRORS: RefCell<HashSet<InjectedError >> = RefCell::new(HashSet::new());
    }

    pub(crate) fn maybe_raise_injected_error(error_type: InjectedError) -> VMResult<()> {
        match INJECTED_ERRORS.with(|injected_errors| injected_errors.borrow_mut().take(&error_type))
        {
            None => Ok(()),
            Some(_) => Err(PartialVMError::new(
                StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
            )
            .with_message(format!("injected error: {:?}", error_type))
            .finish(Location::Undefined)),
        }
    }

    pub fn inject_error_once(error_type: InjectedError) {
        INJECTED_ERRORS.with(|injected_errors| {
            injected_errors.borrow_mut().insert(error_type);
        })
    }
}
