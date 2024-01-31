# Move mutator tool design

## Overview

The Move mutator is a tool that mutates Move source code. It can be used to help test the robustness of Move specifications and unit tests by generating different code versions (mutants).

### What's missing in the Move ecosystem

To create a prototype, we've used existing Move tools and libraries. We've produced a working prototype, so generally, there are no significant gaps in the Move ecosystem. It's sufficient for the mutant generation.

However, some things could be improved. The most important is the specification testing tool as a separate tool. Currently, the Move Prover is used to test specifications, but it can't be used efficiently to test generated mutants.

Move Prover demands to be run inside the Move package. We can't pass to Prover just mutants directory, as it will be unable to find the dependencies. Moreover, it's not also possible to simply copy the mutants inside the existing structure (or create a structure and put the mutants there) as Prover can't handle dozens of files which contain the same modules, the same function names, etc.

There is a need for a tool that will be able to run the Prover, passing it the mutants one by one and then collecting the results. Running the Prover in parallel should be possible, as it will speed up the process. The tool should also be able to handle the Prover output and generate a report. So, that report would be a separate report from the mutation tool.

##  Architecture

The Move mutator tool is composed of the modules that are described below. Each module is responsible for a different application part. Such an approach allows for easy extension and modification. Modules are grouped into logical layers, separating and isolating the application parts.

The Move mutator tool is the heart of the specification verification process, as depending on its quality, the verification process will be more or less efficient. The Move mutator tool is designed to be easily extensible, so it's possible to add new mutation operators, new mutation categories, etc.

The mutation process is separated from the other parts of the verification process. It makes it possible to use mutator output (mutants) to verify the specification's quality using Move Prover and check the completeness and quality of test suites. Users can also use mutants in their own applications and scripts.

### Presentation layer

In this project, the presentation layer execution modules (displaying and fetching data from/to the user) are moved to the `aptos` and `move` command line interfaces.

Mutator itself provides a CLI module, which is integrated into the existing Aptos repository - it handles options that the mutator tool can use.

There are two types of output that the Move mutator tool generates:
- the actual mutants (source code files)
- the report (JSON and text file)

The actual mutants are stored in the output directory (default: `mutants_output/mutants`). The directory structure is the same as in the original source code. The mutant filename consists of the original source filename with mutation information (operator) and the mutant index appended.

The report is generated in the output directory. It's possible to create the report in JSON or text format. The JSON format is used to pass the report to other tools. The text format is used by the user to display the report on the screen.

JSON report format sample:

```json
{
    "mutants": [
      {
        "mutant_path": "mutants_output/shift_0.move",
        "original_file": "third_party/move/move-prover/tests/sources/functional/shift.move",
        "module_name": "module",
        "mutations": [
          {
            "changed_place": {
              "start": 243,
              "end": 245
            },
            "operator_name": "BinaryOperator",
            "old_value": "<<",
            "new_value": ">>"
          }
        ],
        "diff": "--- original\n+++ modified\n@@ -5,7 +5,7 @@\n module 0x42::TestShift {\n\n     fun shiftl_1_correct(x: u64): u64 {\n-        x << 1\n+        x >> 1\n     }\n\n     spec shiftl_1_correct {\n"
      }
    ]
}
```

Text format sample:
```
Mutant path: mutants_output/shift_0.move
Original file: third_party/move/move-prover/tests/sources/functional/shift.move
Module name: module
Mutations:
  Operator: binary_operator_replacement
  Old value: <<
  New value: >>
  Changed place: 243-245
Diff:
--- original
+++ modified
@@ -5,7 +5,7 @@
 module 0x42::TestShift {

     fun shiftl_1_correct(x: u64): u64 {
-        x << 1
+        x >> 1
     }

     spec shiftl_1_correct {

----------------------------------------
```

### Service layer

It contains an API module that exposes function calls to other modules (internal or external).

It is mainly used to expose the `run_move_mutator` function, which is the entry point to the Move mutator tool. It takes the configuration file path as an argument and returns the list of mutants.

### Main logic layer

This layer is responsible for the main logic of the application. The main application layer with crucial components used to generate mutants:

The functions in the `mutate.rs` are responsible for traversing the Abstract Syntax Tree (AST) of the Move source code and searching for places where potential mutation operators can be applied. It has been AST from the `typing` Move compiler module used, but it may not be a big change to traverse other trees (unless the locations pointed by the appropriate structs will match the source code). The `typing` tree has been used because it's the close enough to the original source file but it offers more information about expression types, which may be needed by some mutation operators.
The idea behind that search is as follows: there is a `mutate` function which is the entry point. It takes the AST of a Move program as input and returns a list of mutants (not yet generated). It does this by iterating over the source definitions in the AST and calling the appropriate function to traverse each definition based on its type (Address, Module, or Script). There are several functions to go through different types of AST nodes. 

The main parsing function is the `parse_expression`. It checks the type of the expression and marks that place as the appropriate for the mutation operator to be applied (e.g., binary or unary expressions). Sub-expressions are parsed recursively.

When a new place for mutation is found, new mutant is created. The mutation operator is passed as an argument containing the appropriate AST node. As each node has its own location (`Spanned` structure with `Loc`) it's possible to pass the node without any additional overhead.
This process continues recursively until all nodes in the AST have been visited and all possible mutants have been generated. The mutate function then returns the list of mutants.

Mutations are applied during output generation. Each potential mutation place is run through the `apply` function, which applies the mutation operator to sources. That produces an output file which can be saved.

The `apply` function is the one that handles mutation categories that can exist inside the mutation operators. For example, the binary operator replacement mutation operator has categories like arithmetic, bitwise, and shifts. When the `apply` function is called, it checks if the mutation operator has categories. If so, it chooses an appropriate category based on the expression. ALL mutations within the category are applied.

The above means that the operator tool can produce many mutants within one place in the source code for each mutation. Concrete operators or categories can be filtered using the configuration file described in the data layer section.

Once generated, mutants can be checked to see if they are valid. It's possible to run the Move compiler to check if the mutant is valid, as some of the mutations can create mutants that cannot be compiled properly.

The last module in the main logic layer filters the mutants and reduces the outcome. Filtering is done using percentage parameter which means how many mutants should be rejected.

There is additional behaviour:
- mutator tries to return at least one mutant per file,
- mutator tries to return at least one mutant per mutation operator.

### Data layer

This layer handles data sources - reading Move projects (source code) and configuration files. It also provides data to the other layers.

The configuration file is a JSON or TOML (both supported) file that contains the configuration of the Move mutator tool. It includes information on the project to mutate, mutation operators to use, mutation categories to use, and so on.

Sample configuration file:
```json
{
    "project": {
        "path": "path/to/project",
        "files": [
            "path/to/file1",
            "path/to/file2"
        ]
    },
    "mutation": {
        "operators": [
            "binary_operator_replacement",
            "unary_operator_replacement"
        ],
        "categories": [
            "arithmetic",
            "bitwise",
            "shifts"
        ]
    }
}
```

```toml
[project]
move_sources = ["/path/to/move/source"]
[mutation]
operators = ["operator1", "operator2"]
categories = ["category1", "category2"]
[[individual]]
file = "/path/to/file"
verify_mutants = true
include_functions = ["function1", "function2"]
exclude_functions = ["function3", "function4"]
```

### Cross layer

The layer is used to provide a common function set to other layers. None of their functions is exposed externally. Its crucial components are:
- logging module - gathers logs from other internal components and allows them to be saved for further analysis (e.g., in case of an error).
- IO support - various input/output functions (formatters) and helpers.
- Move compiler API (`compiler.rs`) - various functions to handle Move compiler communication - the ability to compile source files and generate AST from specified files, etc.

## Command line interface integration

Move mutator can be integrated with any command line interface. To do so, CLI application needs to collect arguments and call the `run_move_mutator` function from the `move_mutator` crate.

Currently, the Move mutator tool is integrated with the `move-cli` and `aptos` command line interfaces. There is introduced a new command: `mutate` which allows to pass the mutator arguments. Check README.md for more details.

## Specification testing tool

The Move mutator tool is designed to create mutants only. It does not perform the proving process as it is not the goal of the tool.

The specification verification tool is a tool placed inside the `aptos` repository, which provides additional `aptos` subcommand - `spec-verify` which does the following:
1. Takes arguments both for the Move Prover tool and for the Move mutator tool. It can also read the configuration from the JSON configuration file.
2. Run the Move mutator tool (`mutate`) to generate mutants with the previously specified parameters.
3. Run the Move Prover tool, passing the generated mutants one by one.
4. Collect the results and generate a report.

The report contains information about the generated mutants as well as the killed ones for each tested source file. All the intermediate results are saved in the configurable (default: `mutants_output`) directory.

## Mutation operators

Mutation operators can be applied alone or mixed with other ones. The decision is made based on the configuration file. The default behaviour is to use one mutation operator at a time (per file). It's worth noting that mixing operators within one file will significantly increase the number of generated mutants.

The mutation tool reads the source file(-s) once and then works on the original AST. If any previous mutation changes the original file, it would demand reloading the modified source (as upon change, all current AST locations become outdated), parsing the AST and again for possible mutations. It would be very inefficient. Once mutation places are identified, mutants are generated in reversed order (based on localization) to avoid that. If operator mixing is allowed, they are applied only for non-overlapping expressions.

The above behaviour can be discussed if it's really needed.

The Move mutator tool implements the following mutation operators.

### Binary operator replacement

This mutation operator replaces binary operators with other binary operators. For example, the `+` operator can be replaced with the `-` operator.

Operators are grouped into the following categories:
- arithmetic operators: `+`, `-`, `*`, `/`, `%`
- bitwise operators: `&`, `|`, `^`
- shifts: `<<`, `>>`
- comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
- logical operators: `&&`, `||`

Operators are replaced within the same category. For example, the `+` operator can be replaced with the `-` operator but not the `<<` operator.

Binary operators are never removed, as it would produce invalid code.

The operator tests the conditions in the specifications and test suites.

### Unary operator replacement

This mutation operator replaces unary operators with other unary operators. For example, the `!` operator can be replaced with the space. So, in fact, it removes the operator but without changing the file length.

The operator tests the conditions in the specifications and test suites.

Thanks to the fact that, the operator is replaced with space instead of just removing.

### Literal replacement

This mutation operator replaces literals with other literals. For example, the `0` literal can be replaced with the `1` literal or other random literal, `true` to `false`, etc.

It's possible to choose the type of the literal to be replaced. For example, it's possible to replace only boolean literals.

The operator tests the different conditions in the specifications (like invariants) and test suites.

### Address replacement

This mutation operator replaces addresses with other addresses. For example, the `0x1`/`@std` address can be replaced with the `0x000A` address or another random address.

### Break/continue replacement or deletion

This mutation operator replaces or deletes break/continue statements with other break/continue statements.

## Extending the Move mutator tool

The Move mutator tool is designed to be easily extensible. It's possible to add new mutation operators, new mutation categories and so on. The following sections describe how to do it.

### Adding a new mutation operator

To add a new mutation operator, you need to:
1. Add a new mutation operator to the `MutationOperator` enum in the `operator.rs` file.
2. Add appropriate calls to the `MutationOperator` implementation in the `operator.rs` file.
3. Create new file with the mutation operator implementation in the `operators` directory.
4. Implement mutation logic in the `apply` and `get_file_hash` functions in the newly created file.
5. Update AST traversal code in the `mutate.rs` file - add or modify a place in the AST where the mutation operator should be applied.
6. Add a test for the new mutation operator.

### Adding a new mutation category

Sometimes, it's helpful to group mutation operators into categories. For example, it's useful to group arithmetic operators together. It's possible to do it by adding a new mutation category.

Categories exist only to group mutation operators. They are not used anywhere else. They reside in the file with mutation operator in `operators` directory, inside the `apply` function. To add a new category, simply modify the existing operator, adding or excluding new groups based on the already present code (look at the `BinaryOperator`, `ops` vector).



