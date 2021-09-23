# Duplication Checker Analysis

## Property

No duplication of entries in any pool of a `CompiledModule` or `CompiledScript`.

## Background

The `CompiledModule` (and `CompiledScript`) have a set of pools, e.g. `module.field_handles` which contain structs. These pools need to be checked for duplicated elements.

## Analysis goal

Check that every dataflow of Pool type passes through the checking function: `first_duplicate_element`. Data types to Check: All of the module or script pools:
```
module_handles: Vec<ModuleHandle>
struct_handles: Vec<StructHandle>
function_handles: Vec<FunctionHandle>
field_handles: Vec<FieldHandle>
friend_decls: Vec<ModuleHandle>
struct_def_instantiations: Vec<StructDefInstantiation>
function_instantiations: Vec<FunctionInstantiation>
field_instantiations: Vec<FieldInstantiation>
signatures: Vec<Signature>
identifiers: Vec<Identifier>
address_identifiers: Vec<AccountAddress>
constant_pool: Vec<Constant>
struct_defs: Vec<StructDefinition>
function_defs: Vec<FunctionDefinition>
```
- Note that the `module_handles` and `friend_decls` share the same type so it is not possible to distinguish them in this analysis.

## Analysis definition

1. Entry points:
    1. `verify_module`
    2. `verify_script`
2. Checker: `first_duplicate_element`
3. Exit: Dummy function called at the end of `verify_module` and `verify_script`:
    1. `verify_module_impl_exit`
        ```Rust
        fn verify_module_impl_exit(_module: &'a CompiledModule) -> PartialVMResult<()> {
            Ok(())
        }
        ```
    2. `verify_script_impl_exit`
        ```Rust
        fn verify_script_impl_exit(_module: &'a CompiledScript) -> PartialVMResult<()> {
            Ok(())
        }
        ```
    - Justification: For the DuplicationChecker, the entry points are the same as the exits, thus we need to distinguish the end of the function via a call to this dummy function.
4. Safe functions:
    ```
    check_struct_handles::{closure#0}
    check_function_handles::{closure#0}
    check_function_defintions::{closure#0}
    check_function_defintions::{closure#1}
    check_function_defintions::{closure#2}
    check_struct_definitions::{closure#0}
    check_struct_definitions::{closure#1}
    check_struct_definitions::{closure#2}
    check_struct_definitions::{closure#3}
    verify_script::{closure#0}
    ```
    - Justification: As used in the DuplicationChecker, the above closures lead to two false positives for the analysis (causing both the `struct_defs` and `function_defs` to be identified as `NotCheckedType`). However, these checkers have no unsafe dataflows outside of the DuplicationChecker; they primarily show up in function application on iterators.

## Type relations

The analysis uses input type relations that describe the structure of a compiled module with regards to the data-types-to-check, e.g., `Vec<Signature>, Vec<ModuleHandle>` etc. For example, we provide a relation showing that the `CompiledModule` has a collection of signatures:
```
{
    "kind": "Member",
    "type1": "move_binary_format::CompiledModule",
    "type2": "[file_format::Signature]"
}
```

## Verification conditions for checkers

1. For `first_duplicate_element(iter) -> res: Option<idx>`:
    ```
    res is None =>
        forall idx1 in iter .
            forall idx2 in iter .
                idx1 != idx2 => iter[idx1] != iter[idx2]
    ```

## Required code modifications

1. Add the dummy exit functions.

## Results

All pools are checked:
```
module_handles: Vec<ModuleHandle>
struct_handles: Vec<StructHandle>
function_handles: Vec<FunctionHandle>
field_handles: Vec<FieldHandle>
friend_decls: Vec<ModuleHandle>
struct_def_instantiations: Vec<StructDefInstantiation>
function_instantiations: Vec<FunctionInstantiation>
field_instantiations: Vec<FieldInstantiation>
signatures: Vec<Signature>
identifiers: Vec<Identifier>
address_identifiers: Vec<AccountAddress>
constant_pool: Vec<Constant>
struct_defs: Vec<StructDefinition>
function_defs: Vec<FunctionDefinition>
```
