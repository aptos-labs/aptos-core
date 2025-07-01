// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{module_traversal::TraversalContext, storage::loader::traits::StructDefinitionLoader};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::TypeParameterIndex,
};
use move_core_types::vm_status::StatusCode;
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{
        runtime_types::{DepthFormula, StructIdentifier, StructLayout, Type},
        struct_name_indexing::StructNameIndex,
    },
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

/// Checks depths for instantiated types.
///
/// For structs, stores a cache of formulas. The cache is used for performance (avoid repeated
/// formula construction within a single transaction).
///
/// While a formula is being constructed, also checks for cycles between struct definitions. That
/// is, cases like
/// ```text
/// struct A { b: B, }
/// struct B { a: A, }
/// ```
/// are not allowed and constructing a formula for these types will fail.
pub(crate) struct TypeDepthChecker<'a, T> {
    struct_definition_loader: &'a T,
    /// If set, stores the maximum depth of a type allowed. Otherwise, checker is a no-op.
    maybe_max_depth: Option<u64>,
    /// Caches formulas visited so far.
    formula_cache: RefCell<HashMap<StructNameIndex, DepthFormula>>,
}

impl<'a, T> TypeDepthChecker<'a, T>
where
    T: StructDefinitionLoader,
{
    /// Creates a new depth checker for the specified loader to query struct definitions if needed.
    pub(crate) fn new(struct_definition_loader: &'a T) -> Self {
        let maybe_max_depth = struct_definition_loader
            .runtime_environment()
            .vm_config()
            .max_value_nest_depth;
        Self {
            struct_definition_loader,
            maybe_max_depth,
            formula_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Checks the depth of a type. If the type is too deep, returns an error. Note that the type
    /// must be non-generic, i.e., all type substitutions must be performed. If needed, the check
    /// traverses multiple modules where inner structs and their fields are defined.
    #[allow(dead_code)]
    pub(crate) fn check_depth_of_type(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<()> {
        let max_depth = match self.maybe_max_depth {
            Some(max_depth) => max_depth,
            None => return Ok(()),
        };

        let _timer = VM_TIMER.timer_with_label("check_depth_of_type");

        // Start at 1 since we always call this right before we add a new node to the value's depth.
        self.recursive_check_depth_of_type(gas_meter, traversal_context, ty, max_depth, 1)?;
        Ok(())
    }
}

// Private interfaces below.
impl<'a, T> TypeDepthChecker<'a, T>
where
    T: StructDefinitionLoader,
{
    /// Recursive implementation of type depth checks. For structs, computes and caches the depth
    /// formula which is solved to get the actual depth and check it against the maximum allowed
    /// value.
    fn recursive_check_depth_of_type(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
        max_depth: u64,
        depth: u64,
    ) -> PartialVMResult<u64> {
        macro_rules! visit_struct {
            ($idx:expr) => {{
                // Keeps track of formulas visited during construction (to detect cycles). Should
                // always be empty before and after the formula is constructed end-to-end.
                let mut currently_visiting = HashSet::new();
                let formula = self.calculate_struct_depth_formula(
                    gas_meter,
                    traversal_context,
                    &mut currently_visiting,
                    $idx,
                )?;
                if !currently_visiting.is_empty() {
                    let struct_name = self.get_struct_name($idx)?;
                    return Err(PartialVMError::new_invariant_violation(format!(
                        "Constructing a formula for {}::{}::{} has non-empty visiting set",
                        struct_name.module.address, struct_name.module.name, struct_name.name
                    )));
                }
                formula
            }};
        }

        macro_rules! check_depth {
            ($additional_depth:expr) => {{
                let new_depth = depth.saturating_add($additional_depth);
                if new_depth > max_depth {
                    return Err(PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED));
                } else {
                    new_depth
                }
            }};
        }

        let ty_depth = match ty {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::Address
            | Type::Signer => check_depth!(0),
            Type::Reference(ty) | Type::MutableReference(ty) => self
                .recursive_check_depth_of_type(
                    gas_meter,
                    traversal_context,
                    ty,
                    max_depth,
                    check_depth!(1),
                )?,
            Type::Vector(ty) => self.recursive_check_depth_of_type(
                gas_meter,
                traversal_context,
                ty,
                max_depth,
                check_depth!(1),
            )?,
            Type::Struct { idx, .. } => {
                let formula = visit_struct!(idx);
                check_depth!(formula.solve(&[]))
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                let ty_arg_depths = ty_args
                    .iter()
                    .map(|ty| {
                        self.recursive_check_depth_of_type(
                            gas_meter,
                            traversal_context,
                            ty,
                            max_depth,
                            check_depth!(0),
                        )
                    })
                    .collect::<PartialVMResult<Vec<_>>>()?;

                let formula = visit_struct!(idx);
                check_depth!(formula.solve(&ty_arg_depths))
            },
            Type::Function { args, results, .. } => {
                let mut ty_max_depth = depth;
                for ty in args.iter().chain(results) {
                    ty_max_depth = ty_max_depth.max(self.recursive_check_depth_of_type(
                        gas_meter,
                        traversal_context,
                        ty,
                        max_depth,
                        check_depth!(1),
                    )?);
                }
                ty_max_depth
            },
            Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("Type parameter should be fully resolved".to_string()),
                )
            },
        };

        Ok(ty_depth)
    }

    /// Calculates the depth formula for a (possibly generic) struct or enum. Returns an error if
    /// the struct definition is recursive: i.e., some struct A uses struct B that uses struct A.
    fn calculate_struct_depth_formula(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        currently_visiting: &mut HashSet<StructNameIndex>,
        idx: &StructNameIndex,
    ) -> PartialVMResult<DepthFormula> {
        // If the struct is being visited, we found a recursive definition.
        if currently_visiting.contains(idx) {
            let struct_name = self.get_struct_name(idx)?;
            let msg = format!(
                "Definition of struct {}::{}::{} is recursive: failed to construct its depth formula",
                struct_name.module.address, struct_name.module.name, struct_name.name
            );
            return Err(
                PartialVMError::new(StatusCode::RUNTIME_CYCLIC_MODULE_DEPENDENCY).with_message(msg),
            );
        }

        // Otherwise, check if we cached it.
        if let Some(formula) = self.formula_cache.borrow().get(idx) {
            return Ok(formula.clone());
        }

        // Struct has not been visited, mark as being visited.
        assert!(currently_visiting.insert(*idx));

        // Recursively visit its fields to construct the formula.
        let struct_definition = self.struct_definition_loader.load_struct_definition(
            gas_meter,
            traversal_context,
            idx,
        )?;

        let formulas = match &struct_definition.layout {
            StructLayout::Single(fields) => fields
                .iter()
                .map(|(_, field_ty)| {
                    self.calculate_type_depth_formula(
                        gas_meter,
                        traversal_context,
                        currently_visiting,
                        field_ty,
                    )
                })
                .collect::<PartialVMResult<Vec<_>>>()?,
            StructLayout::Variants(variants) => variants
                .iter()
                .flat_map(|variant| variant.1.iter().map(|(_, ty)| ty))
                .map(|field_ty| {
                    self.calculate_type_depth_formula(
                        gas_meter,
                        traversal_context,
                        currently_visiting,
                        field_ty,
                    )
                })
                .collect::<PartialVMResult<Vec<_>>>()?,
        };
        let formula = DepthFormula::normalize(formulas);

        // Add the formula to cace list and remove it from the currently visited set.
        assert!(currently_visiting.remove(idx));
        let prev = self
            .formula_cache
            .borrow_mut()
            .insert(*idx, formula.clone());
        if prev.is_some() {
            // Clear cache if there is an invariant violation, to be safe.
            self.formula_cache.borrow_mut().clear();
            let struct_name = self.get_struct_name(idx)?;
            let msg = format!(
                "Depth formula for struct {}::{}::{} is already cached",
                struct_name.module.address, struct_name.module.name, struct_name.name
            );
            return Err(PartialVMError::new_invariant_violation(msg));
        }

        Ok(formula)
    }

    /// Calculates the depth formula of the specified [Type]. There are no constraints on the type,
    /// it can also be a generic type parameter.
    fn calculate_type_depth_formula(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        currently_visiting: &mut HashSet<StructNameIndex>,
        ty: &Type,
    ) -> PartialVMResult<DepthFormula> {
        Ok(match ty {
            Type::Bool
            | Type::U8
            | Type::U64
            | Type::U128
            | Type::Address
            | Type::Signer
            | Type::U16
            | Type::U32
            | Type::U256 => DepthFormula::constant(1),
            Type::Vector(ty) => self
                .calculate_type_depth_formula(gas_meter, traversal_context, currently_visiting, ty)?
                .scale(1),
            Type::Reference(ty) | Type::MutableReference(ty) => self
                .calculate_type_depth_formula(gas_meter, traversal_context, currently_visiting, ty)?
                .scale(1),
            Type::TyParam(ty_idx) => DepthFormula::type_parameter(*ty_idx),
            Type::Struct { idx, .. } => {
                let struct_formula = self.calculate_struct_depth_formula(
                    gas_meter,
                    traversal_context,
                    currently_visiting,
                    idx,
                )?;
                debug_assert!(struct_formula.terms.is_empty());
                struct_formula.scale(1)
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                let ty_arg_map = ty_args
                    .iter()
                    .enumerate()
                    .map(|(idx, ty)| {
                        let var = idx as TypeParameterIndex;
                        Ok((
                            var,
                            self.calculate_type_depth_formula(
                                gas_meter,
                                traversal_context,
                                currently_visiting,
                                ty,
                            )?,
                        ))
                    })
                    .collect::<PartialVMResult<BTreeMap<_, _>>>()?;
                let struct_formula = self.calculate_struct_depth_formula(
                    gas_meter,
                    traversal_context,
                    currently_visiting,
                    idx,
                )?;
                struct_formula.subst(ty_arg_map)?.scale(1)
            },
            Type::Function {
                args,
                results,
                abilities: _,
            } => {
                let inner_formulas = args
                    .iter()
                    .chain(results)
                    .map(|ty| {
                        self.calculate_type_depth_formula(
                            gas_meter,
                            traversal_context,
                            currently_visiting,
                            ty,
                        )
                    })
                    .collect::<PartialVMResult<Vec<_>>>()?;
                DepthFormula::normalize(inner_formulas).scale(1)
            },
        })
    }

    /// Returns the struct name for the specified index.
    fn get_struct_name(&self, idx: &StructNameIndex) -> PartialVMResult<Arc<StructIdentifier>> {
        self.struct_definition_loader
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)
    }
}

// Test-only interfaces below.
#[cfg(test)]
impl<'a, T> TypeDepthChecker<'a, T>
where
    T: StructDefinitionLoader,
{
    /// Creates a new depth checker for the specified loader and with specified maximum depth.
    fn new_with_max_depth(struct_definition_loader: &'a T, max_depth: u64) -> Self {
        Self {
            struct_definition_loader,
            maybe_max_depth: Some(max_depth),
            formula_cache: RefCell::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        module_traversal::TraversalStorage,
        storage::loader::test_utils::{generic_struct_ty, struct_ty, MockStructDefinitionLoader},
    };
    use claims::{assert_err, assert_ok};
    use move_vm_types::gas::UnmeteredGasMeter;

    #[test]
    fn test_struct_definition_not_found_cache_consistency() {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = MockStructDefinitionLoader::default();

        // Create structs A and B with definition of C missing:
        //
        // struct A {}
        // struct B { c: C, }
        let a = loader.get_struct_identifier("A");
        let b = loader.get_struct_identifier("B");
        let c = loader.get_struct_identifier("C");
        loader.add_struct("A", vec![]);
        loader.add_struct("B", vec![("c", struct_ty(c))]);

        let checker = TypeDepthChecker::new(&loader);
        let mut currently_visiting = HashSet::new();

        assert_ok!(checker.calculate_struct_depth_formula(
            &mut gas_meter,
            &mut traversal_context,
            &mut currently_visiting,
            &a
        ));
        assert!(currently_visiting.is_empty());
        assert_eq!(checker.formula_cache.borrow().len(), 1);

        let err = assert_err!(checker.calculate_struct_depth_formula(
            &mut gas_meter,
            &mut traversal_context,
            &mut currently_visiting,
            &b
        ));
        assert_eq!(err.major_status(), StatusCode::LINKER_ERROR);
        assert_eq!(checker.formula_cache.borrow().len(), 1);
    }

    #[test]
    fn test_runtime_cyclic_module_dependency() {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = MockStructDefinitionLoader::default();

        // No cycles (structs and enums):
        //
        // struct A {}
        // enum B {
        //   V1 { x: bool, },
        //   V2 { a: A, },
        // }
        let a = loader.get_struct_identifier("A");
        let b = loader.get_struct_identifier("B");
        loader.add_struct("A", vec![]);
        loader.add_enum("B", vec![
            ("V1", vec![("x", Type::Bool)]),
            ("V2", vec![("a", struct_ty(a))]),
        ]);

        // Cycles between structs C and D, and between E and itself:
        //
        // struct C { d: D, }
        // struct D { c: C, }
        // struct E { e: E, }
        let c = loader.get_struct_identifier("C");
        let d = loader.get_struct_identifier("D");
        let e = loader.get_struct_identifier("E");
        loader.add_struct("C", vec![("d", struct_ty(d))]);
        loader.add_struct("D", vec![("c", struct_ty(c))]);
        loader.add_struct("E", vec![("e", struct_ty(e))]);

        // Cycles between enums F and G.
        //
        // enum F {
        //   V0 { g: G, },
        // }
        // enum G {
        //   V0 { fs: vector<F>, },
        // }
        let f = loader.get_struct_identifier("F");
        let g = loader.get_struct_identifier("G");
        loader.add_enum("F", vec![("V0", vec![("g", struct_ty(g))])]);
        loader.add_enum("G", vec![("V0", vec![(
            "fs",
            Type::Vector(triomphe::Arc::new(struct_ty(f))),
        )])]);

        let checker = TypeDepthChecker::new(&loader);
        let mut currently_visiting = HashSet::new();

        for idx in [a, b] {
            assert_ok!(checker.calculate_struct_depth_formula(
                &mut gas_meter,
                &mut traversal_context,
                &mut currently_visiting,
                &idx
            ));
            assert!(currently_visiting.is_empty());
        }

        assert_eq!(checker.formula_cache.borrow().len(), 2);

        for idx in [c, d, e, f, g] {
            let err = assert_err!(checker.calculate_struct_depth_formula(
                &mut gas_meter,
                &mut traversal_context,
                &mut currently_visiting,
                &idx
            ));
            assert_eq!(
                err.major_status(),
                StatusCode::RUNTIME_CYCLIC_MODULE_DEPENDENCY
            );
        }
        assert_eq!(checker.formula_cache.borrow().len(), 2);
    }

    #[test]
    fn test_runtime_cyclic_module_dependency_generic() {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = MockStructDefinitionLoader::default();

        // Simple struct to store in the checker cache to check cache consistency.
        let a = loader.get_struct_identifier("A");
        loader.add_struct("A", vec![]);

        // Cycle between generic struct B and C:
        //
        // struct B<T> { c: C<T> }
        // struct C<T> { x: T, b: B<T> }
        let b = loader.get_struct_identifier("B");
        let c = loader.get_struct_identifier("C");
        loader.add_struct("B", vec![(
            "c",
            generic_struct_ty(c, vec![Type::TyParam(0)]),
        )]);
        loader.add_struct("C", vec![
            ("x", Type::TyParam(0)),
            ("b", generic_struct_ty(b, vec![Type::TyParam(0)])),
        ]);

        // Cycle between generic enum and generic struct:
        //
        // struct D<T> { x: T, e: E<u8>, }
        // enum E<T> {
        //   V0 { ds: vector<D<u8>>, },
        // }
        let d = loader.get_struct_identifier("D");
        let e = loader.get_struct_identifier("E");
        loader.add_struct("D", vec![
            ("x", Type::TyParam(0)),
            ("e", generic_struct_ty(e, vec![Type::U8])),
        ]);
        loader.add_enum("E", vec![("V0", vec![(
            "ds",
            Type::Vector(triomphe::Arc::new(struct_ty(d))),
        )])]);

        let checker = TypeDepthChecker::new(&loader);
        let mut currently_visiting = HashSet::new();

        assert_ok!(checker.calculate_struct_depth_formula(
            &mut gas_meter,
            &mut traversal_context,
            &mut currently_visiting,
            &a
        ));

        assert!(currently_visiting.is_empty());
        assert_eq!(checker.formula_cache.borrow().len(), 1);

        for idx in [b, c, d, e] {
            let err = assert_err!(checker.calculate_struct_depth_formula(
                &mut gas_meter,
                &mut traversal_context,
                &mut currently_visiting,
                &idx,
            ));
            assert_eq!(
                err.major_status(),
                StatusCode::RUNTIME_CYCLIC_MODULE_DEPENDENCY
            );
            assert_eq!(checker.formula_cache.borrow().len(), 1);
        }
    }

    #[test]
    fn test_runtime_non_cyclic_module_dependencies_generic() {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = MockStructDefinitionLoader::default();

        // These structs use definitions recursively but in type arguments, which is safe.
        //
        // struct A<T> { x: T, }
        // struct B { xs: A<A<A<u8>>>, }
        // struct C<T> { x: T, }
        // struct D<T> { xs: A<C<A<u8>>>, }

        let a = loader.get_struct_identifier("A");
        let b = loader.get_struct_identifier("B");
        let c = loader.get_struct_identifier("C");
        let d = loader.get_struct_identifier("D");

        let a_u8_ty = generic_struct_ty(a, vec![Type::U8]);
        let a_a_u8_ty = generic_struct_ty(a, vec![a_u8_ty.clone()]);
        let a_a_a_u8_ty = generic_struct_ty(a, vec![a_a_u8_ty]);
        let c_a_u8_ty = generic_struct_ty(c, vec![a_u8_ty]);
        let a_c_a_u8_ty = generic_struct_ty(a, vec![c_a_u8_ty]);

        loader.add_struct("A", vec![("x", Type::TyParam(0))]);
        loader.add_struct("B", vec![("xs", a_a_a_u8_ty)]);
        loader.add_struct("C", vec![("x", Type::TyParam(0))]);
        loader.add_struct("D", vec![("xs", a_c_a_u8_ty)]);

        let checker = TypeDepthChecker::new(&loader);
        let mut currently_visiting = HashSet::new();

        for idx in [a, b, c, d] {
            assert_ok!(checker.calculate_struct_depth_formula(
                &mut gas_meter,
                &mut traversal_context,
                &mut currently_visiting,
                &idx,
            ));
            assert!(currently_visiting.is_empty());
        }

        assert_eq!(checker.formula_cache.borrow().len(), 4);
    }

    #[test]
    fn test_runtime_non_cyclic_module_dependencies() {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = MockStructDefinitionLoader::default();

        let a = loader.get_struct_identifier("A");
        let b = loader.get_struct_identifier("B");
        let c = loader.get_struct_identifier("C");

        // These structs are not recursive.
        //
        // struct C {}
        // struct B { c: C, }
        // struct A { c: C, b: B, }

        loader.add_struct("C", vec![]);
        loader.add_struct("B", vec![("c", struct_ty(c))]);
        loader.add_struct("A", vec![("c", struct_ty(c)), ("b", struct_ty(b))]);

        for idx in [a, b, c] {
            let checker = TypeDepthChecker::new(&loader);
            let mut currently_visiting = HashSet::new();

            assert_ok!(checker.calculate_struct_depth_formula(
                &mut gas_meter,
                &mut traversal_context,
                &mut currently_visiting,
                &idx,
            ));
        }
    }

    #[test]
    fn test_ty_to_deep() {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = MockStructDefinitionLoader::default();

        let a = loader.get_struct_identifier("A");
        let b = loader.get_struct_identifier("B");
        let c = loader.get_struct_identifier("C");

        loader.add_struct("C", vec![("dummy", Type::Bool)]);
        loader.add_struct("B", vec![("c", struct_ty(c))]);
        loader.add_struct("A", vec![("b", struct_ty(b))]);

        let checker = TypeDepthChecker::new_with_max_depth(&loader, 2);

        assert_ok!(checker.check_depth_of_type(&mut gas_meter, &mut traversal_context, &Type::U8));

        let vec_u8_ty = Type::Vector(triomphe::Arc::new(Type::U8));
        assert_ok!(checker.check_depth_of_type(&mut gas_meter, &mut traversal_context, &vec_u8_ty));

        let vec_vec_u8_ty = Type::Vector(triomphe::Arc::new(vec_u8_ty.clone()));
        assert_err!(checker.check_depth_of_type(
            &mut gas_meter,
            &mut traversal_context,
            &vec_vec_u8_ty
        ));
        let ref_vec_u8_ty = Type::Reference(Box::new(vec_u8_ty));
        assert_err!(checker.check_depth_of_type(
            &mut gas_meter,
            &mut traversal_context,
            &ref_vec_u8_ty
        ));

        assert_ok!(checker.check_depth_of_type(
            &mut gas_meter,
            &mut traversal_context,
            &struct_ty(c)
        ));
        assert_err!(checker.check_depth_of_type(
            &mut gas_meter,
            &mut traversal_context,
            &struct_ty(b)
        ));
        assert_err!(checker.check_depth_of_type(
            &mut gas_meter,
            &mut traversal_context,
            &struct_ty(a)
        ));
    }
}
