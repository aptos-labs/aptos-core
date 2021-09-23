# Overview

We present two example analyses using `mirai-dataflow`:
1. An analysis on the move-binary-format crate's [BoundsChecker](./BoundsChecker.md).
2. An analysis on the bytecode-verifier crate's [DuplicationChecker](./DuplicationChecker.md).

## Environment

All analysis results were captured using the following software versions:
- The Diem commit that this file was published with (after `92a19a426bae04199f2ca0845dfb19fdee4740a6`).
- The commit of MIRAI in use is `49c8add3706d49f82d96f3ace2c01c738ea04dc2`.
- The version of Soufflé in use is `2.1-123-g1f664173b`.
- The version of Differential Datalog in use is `v0.42.0 (f2824fec7b86c84486ec34b8880bd590678c0765)`.

Please note that MIRAI's [is_excluded](https://github.com/facebookexperimental/MIRAI/blob/49c8add3706d49f82d96f3ace2c01c738ea04dc2/checker/src/callbacks.rs#L145) function will have to be modified _before_ installing MIRAI to not exclude the `move-binary-format` and `bytecode-verifier` crates:
1. For `move-binary-format` comment out [this line](https://github.com/facebookexperimental/MIRAI/blob/49c8add3706d49f82d96f3ace2c01c738ea04dc2/checker/src/callbacks.rs#L215).
2. For the `bytecode-verifier` comment out [this line](https://github.com/facebookexperimental/MIRAI/blob/49c8add3706d49f82d96f3ace2c01c738ea04dc2/checker/src/callbacks.rs#L149).

## Example Structure

The example analyses have the following structure:
1. Property
2. Background
3. Analysis goal
4. Analysis definition
5. Type relations
6. Verification conditions for checkers
7. Required code modifications
8. Results

The **Property** section describes the property we want the analysis to check for.

The **Background** section explains relevant background for the property.

The **Analysis goal** section translates the property into a checked / unchecked dataflow problem statement, and states the data types that should be checked.

The **Analysis definition** section identifies the entry point(s), checking function(s), and exit point(s). These correspond exactly to the `"node_type"` configuration of the analysis.

The **Type relations** section describes the necessary input type relations needed to describe the data types that will be checked.

The **Verification conditions for checkers** section states invariants that the checking function(s) must uphold in order for the analysis to imply satisfaction of the property.

The **Required code modifications** section describes modifications that have (or had) to be performed to get the analysis to function correctly.

Finally, the **Results** section describes the results of the analysis; e.g., which of the data-types-to-check are identified as being checked.

## Running an example analysis

The results presented in one of the analysis documents can be replicated by following these steps:
1. Ensure that installed versions of software match those listed under the "Environment" heading of _this_ document.
2. Ensure that a configuration file is present for the analysis and matches what is listed in the "Analysis definition" section of that analysis document. For the above examples, three configuration files should be present in the `config` folder:
    ```
    BoundsChecker_config.json
    DuplicationChecker_config.json
    CompiledModule_type_relations.json
    ```
3. Make code modifications as described in the "Required code modifications" section of the analysis document.
4. Run the analysis using the provided configuration. For example, to run the BoundsChecker analysis:
    ```
    cargo run -- ../../move-binary-format config/BoundsChecker_config.json --type-relations-path config/CompiledModule_type_relations.json
    ```
5. Review the results in `output/decoded.json` to ensure that they match those presented in the analysis document.
