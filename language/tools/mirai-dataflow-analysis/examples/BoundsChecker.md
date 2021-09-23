# Bounds Checker Analysis

## Property

All module indexes fall within bounds of a corresponding pool.

## Background

The `CompiledModuleMut` has a set of pools, e.g. `module.field_handles` which contain structs that individually need to be checked. These structs are identified via indexes into their respective pools.

## Analysis goal

Check that every dataflow of `Index` type passes through one of the these three checking functions:

1. `check_bounds_impl`
2. `check_code_unit_bounds_impl`
3. `check_type_parameter`

Data types to Check: Data of `Index` type:
```
ModuleHandleIndex,
StructHandleIndex,
FunctionHandleIndex,
FieldHandleIndex,
StructDefInstantiationIndex,
FunctionInstantiationIndex,
FieldInstantiationIndex,
IdentifierIndex,
AddressIdentifierIndex,
ConstantPoolIndex,
SignatureIndex,
StructDefinitionIndex,
FunctionDefinitionIndex
```

## Analysis definition

1. Entry: `verify_impl`
    - Justification: Both public entry points of the bounds checker, `verify_script` and `verify_module` subsequently call `verify_impl`.
2. Checkers:
    1. `check_bounds_impl`
    2. `check_code_unit_bounds_impl`
    3. `check_type_parameter`
3. Exit: Dummy function called at the end of `verify_impl`:
    ```Rust
    fn verify_impl_exit(&self) -> PartialVMResult<()> {
        Ok(())
    }
    ```
    - Justification: For the BoundsChecker, the entry point is the same as the exit (`verify_impl`), thus we need to distinguish the end of the function via a call to this dummy function.
4. Safe functions:
    ```
    get_locals,
    offset_out_of_bounds
    ```
    - Justification: These functions are false positives of the analysis if not explicitly marked as safe. They are safe because they are utility functions that have dataflow within the context of the BoundsChecker.

## Type relations

The analysis uses input type relations that describe the structure of a compiled module with regards to the data-types-to-check, e.g., `ConstantPoolIndex, IdentifierIndex`, etc. For example, we provide a relation showing that the `CompiledModule` has a collection of `ModuleHandle`s that each have `IdentifierIndex`s:
```
{
    "kind": "Member",
    "type1": "move_binary_format::CompiledModule",
    "type2": "[file_format::ModuleHandle]"
},
{
    "kind": "Member",
    "type1": "file_format::ModuleHandle",
    "type2": "file_format::IdentifierIndex"
},
```

## Verification conditions for checkers

1. For `check_bounds_impl(pool: &[T], idx: I)`
    - Invariant: `idx >= pool.len() ==> Err`
2. For `check_code_unit_bounds_impl(pool: &[T], idx: I)`
    - Invariant: `idx >= pool.len() ==> Err`
5. For `check_type_parameter(ty: &SignatureToken, type_param_count: usize)`
    - Invariant: `exists ty == TypeParameter(idx) in ty . idx >= type_param_count ==> Err`

## Required code modifications

1. Add the exit dummy function.
2. Separate `verify_impl` loops into individual functions. This is already done as of Diem commit `5122a1d9994350d0c2de7acfe4b44c59e157d2fa`. Example:
    - Before:
        ```Rust
        fn verify_impl(...) {
            ...
            for signature in self.view.signatures() {
                self.check_signature(signature)?
            }
            ...
        }
        ```
    - After
        ```Rust
        fn verify_impl(...) {
            ...
            self.check_signatures(signature)?;
            ...
        }

        fn check_signatures(&self) -> PartialVMResult<()> {
            for signature in self.view.signatures() {
                self.check_signature(signature)?
            }
            Ok(())
        }
        ```

    - Justification: False positive occurs if this is not done (
    `NotCheckedType` with zero loop iterations):
        - We cannot establish a dominance relationship between one of the functions in loops (e.g., `self.check_signature` and the exit. This is because the loop may execute zero times, in which case `self.check_signature` will not be called.
        - Technically a true positive in the sense that there is a path to the exit where the dataflow is not checked, but actually a false positive because the loop only does not execute if there are no members of that dataflow to check in the first place.

## Results

All `Index` types are checked:
```
ModuleHandleIndex,
StructHandleIndex,
FunctionHandleIndex,
FieldHandleIndex,
StructDefInstantiationIndex,
FunctionInstantiationIndex,
FieldInstantiationIndex,
IdentifierIndex,
AddressIdentifierIndex,
ConstantPoolIndex,
SignatureIndex,
StructDefinitionIndex,
FunctionDefinitionIndex
```
