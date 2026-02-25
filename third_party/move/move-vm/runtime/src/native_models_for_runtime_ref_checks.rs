// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashSet};

/// For each native function that returns references, we maintain a model of which
/// of the returned references are derived from which of the input reference
/// parameters.
///
/// The model is represented as a vector of input parameter indices (0-based).
/// There is one entry in the vector for each returned reference.
/// That entry says which input parameter index the returned reference is derived from.
/// Entries should only refer to input parameters that are references.
/// Note that there are no entries for returned values that are not references.
///
/// For example, consider a native function with signature:
/// `native fun foo(x: u64, y1: &u64, y2: &mut u64, y3: &mut u64): (u64, &u64, &mut u64)`
/// A model `[1, 3]` indicates that, for the 3 return values in order:
/// - the first return value (u64) is not a reference, so there is no entry for it
/// - the second return value (&u64) is derived from parameter `y1`
/// - the third return value (&mut u64) is derived from the parameter `y3`
///
/// Notes for possible future extensions:
/// - We currently only use the module and function names as keys to identify native
///   functions, and not include addresses. If there is a need in the future to
///   distinguish between different native-publishable addresses, we can extend the
///   key to include addresses.
/// - The model interface currently only supports one return value derivation per
///   input reference, as this is sufficient for the existing native functions.
///   If we need to support multiple (exclusive) derivations from the same input
///   reference parameter, we can extend each model entry to be a tuple
///   (input_param_index, derivation_label), where `derivation_label` is 0, 1, ..
///   for each distinct derivation from the same input parameter.
///   Currently, we just use the label `0` for all derivations, as there is only
///   one derivation per input parameter.
#[derive(Clone)]
pub struct NativeRuntimeRefChecksModel {
    /// Collection of models for native functions returning references.
    /// The key is (module_name, function_name).
    /// The value is the model vector as described above.
    models: BTreeMap<(&'static str, &'static str), Vec<usize>>,
}

impl Default for NativeRuntimeRefChecksModel {
    /// Create default models for native functions that return references.
    fn default() -> Self {
        // First return value is a reference derived from the first reference parameter.
        // It is the only return value that is a reference.
        let single_return_derived_from_first_ref_param = vec![0];
        let models = BTreeMap::from([
            (
                ("box", "borrow_boxed"),
                single_return_derived_from_first_ref_param.clone(),
            ),
            (
                ("box", "borrow_boxed_mut"),
                single_return_derived_from_first_ref_param.clone(),
            ),
            (
                ("signer", "borrow_address"),
                single_return_derived_from_first_ref_param.clone(),
            ),
            (
                ("table", "borrow_box"),
                single_return_derived_from_first_ref_param.clone(),
            ),
            (
                ("table", "borrow_box_mut"),
                single_return_derived_from_first_ref_param,
            ),
        ]);
        let me = Self { models };
        debug_assert!(
            me.models.values().all(|m| Self::no_duplicates(m)),
            "duplicate derivations in a native model"
        );
        me
    }
}

impl NativeRuntimeRefChecksModel {
    /// Add a runtime ref checks `model` for a native function.
    /// For the semantics of `model`, see the struct documentation.
    /// The native function is identified by its module and function names.
    pub fn add_model_for_native_function(
        &mut self,
        module_name: &'static str,
        function_name: &'static str,
        model: Vec<usize>,
    ) {
        debug_assert!(
            Self::no_duplicates(&model),
            "duplicate derivations in the native model"
        );
        self.models.insert((module_name, function_name), model);
    }

    /// Get the runtime ref checks model for a native function, if it exists.
    pub fn get_model_for_native_function(
        &self,
        module_name: &str,
        function_name: &str,
    ) -> Option<Vec<usize>> {
        self.models.get(&(module_name, function_name)).cloned()
    }

    #[allow(dead_code)]
    fn no_duplicates(model: &[usize]) -> bool {
        let hash_set = HashSet::<usize>::from_iter(model.iter().cloned());
        hash_set.len() == model.len()
    }
}
