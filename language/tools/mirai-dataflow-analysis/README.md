# MIRAI Dataflow Analyzer

This tool (`mirai-dataflow`) provides an end-to-end method for running a dataflow analysis over a Rust crate.
Internally, this tool uses [MIRAI](https://github.com/facebookexperimental/MIRAI) to generate a call graph of a Rust crate, as well as a Datalog
representation of that call graph. Then, a dataflow analysis is run over the call graph using either
[Differential Datalog](https://github.com/vmware/differential-datalog) or [Soufflé](https://souffle-lang.github.io/) and decoded results are presented.

## Examples
Please see the included [examples](examples/examples.md) for notes on analysis construction, execution, and results.

## Installation

This tool depends on the presence of MIRAI, correctly installed on your system, as well as
either an installation of either Differential Datalog or Soufflé Datalog. Thus, to get a complete
setup, please follow the steps below:

1. Install MIRAI by following the instructions in its [readme](https://github.com/facebookexperimental/MIRAI#readme).
2. Install Soufflé and / or Differential Datalog.
3. Set the nightly version of Rust for this crate using `rustup override set nightly-YYYY-MM-DD`.
    - See [rust-toolchain](./rust_toolchain)) to determine the current compatible version, which in turn
    must match the version [currently in use by MIRAI](https://github.com/facebookexperimental/MIRAI/blob/main/rust-toolchain).

Please note that the MIRAI's [is_excluded](https://github.com/facebookexperimental/MIRAI/blob/49c8add3706d49f82d96f3ace2c01c738ea04dc2/checker/src/callbacks.rs#L145) function may have to be modified _before_ installing MIRAI to not exclude certain crates.

### Differential-Datalog-specific step

If Differential Datalog is being used, please additionally use Differential Datalog to build `analyses/ddlog.dl`.
```
$ cd analyses/
$ ddlog -i ddlog.dl
$ cd ddlog_ddlog && cargo build --release
```
This will create a Rust crate (`analyses/ddlog_ddlog`) that includes a CLI that the `mirai-dataflow`
interfaces with to perform its analysis.

## Configuration

`mirai-dataflow` is configured via a combination of CLI arguments and a configuration file.

### CLI

`mirai-dataflow` exposes the following CLI (viewable via the `--help` flag):
```
Rust dataflow analyzer built on MIRAI.

USAGE:
    mirai-dataflow-analysis [FLAGS] [OPTIONS] <crate-path> <config-path>

FLAGS:
        --call-graph-only    Only produce a call graph (no analysis)
    -h, --help               Prints help information
    -n, --no-rebuild         Do not rebuild the crate before analysis
    -r, --reanalyze          Rerun the Datalog analysis without running MIRAI
    -V, --version            Prints version information

OPTIONS:
    -d, --datalog-backend <datalog-backend>            Datalog backend to use (DifferentialDatalog | Souffle)
    -t, --type-relations-path <type-relations-path>    Path to input type relations

ARGS:
    <crate-path>     Path to the crate to analyze
    <config-path>    Path to configuration file
```

The `crate-path` is the path to the crate you want to analyze. The `config-path` is a path to
a configuration file (explained below). Finally, the `type-relations-path` is a path to a set of
input type relations (see the call graph generator's
[documentation](https://github.com/facebookexperimental/MIRAI/blob/main/documentation/CallGraph.md#type-relations)
for details).

### Config File

One of the required CLI arguments is a path to a configuration file. This configuration file tells
`mirai-dataflow` how the generated call graph should be preprocessed before analysis.

The configuration file is a JSON file following this schema:
```
{
    "reductions": List[Reduction],
    "included_crates": List[String],
    "node_types": {
        "entry": List[String],
        "checker: List[String],
        "safe": List[String],
        "exit": List[String]
    }
}
```
where a Reduction is one of:
```
{"Slice": String}
"Fold"
"Clean"
"Deduplicate"
```

Please see the call graph generator's
[documentation](https://github.com/facebookexperimental/MIRAI/blob/main/documentation/CallGraph.md)
for more details on the "reductions" and "included_crates" fields.

#### Node Type Specifications

The `node_type` configuration is of particular importance as it is used to inform the analysis
of entry points, exit points, checkers, and endpoints that can be safely ignored.
Please see below for an explanation of each of these types:

1. Entry: This is a list of names of function that are considered entry points of the crate.
2. Checker: Names of functions that are considered checking functions of the crate.
3. Safe: Names of functions that are not checking functions, but are "safe" if they are endpoints
where an unchecked dataflow ends. This field is typically used to exclude function endpoints that
are false positives for an analysis.
4. Exit: Names of functions that are considered the exit points of the crate.

These node types are converted into datalog input relations that are provided to the analysis
as well as the call graph input relations.
For Soufflé, a file name `NodeType.facts` is generated with relations of the form:
1. Entry: `{node_id},0`
2. Checker: `{node_id},1`
3. Safe: `{node_id},1`
4. Exit: `{node_id},2`

For Differential Datalog, input relations are added directly to the call graph input relations file:
1. Entry: `NodeType({node_id},Entry)`
2. Checker: `NodeType({node_id},Checker)`
3. Safe: `NodeType({node_id},Checker)`
4. Exit: `NodeType({node_id},Exit)`

Note that both Checker and Safe are assigned to Checker. This is a valid optimization as semantically
they are the same in the default analysis, but if an analysis assigns a different meaning to them
this can be modified.

#### Example Configuration

Below is an example of a valid configuration file:
```
{
    "reductions": [
        {"Slice": "verify_impl"},
        "Fold",
        "Clean"
    ],
    "included_crates": ["check_bounds"],
    "node_types": {
        "entry": [
            "verify_impl"
        ],
        "checker": [
            "check_bounds_impl",
            "check_code_unit_bounds_impl",
            "check_type_parameter"
        ],
        "exit": [
            "verify_impl_exit"
        ]
    }
}
```

## Usage

Having configured `mirai-dataflow` as explained above, the analysis can then be run:
```
$ cargo run -- ../../move-binary-format config/call_graph_config.json --type-relations-path=config/type_relations.json
```

`mirai-dataflow` will then execute MIRAI on the crate and then execute the Datalog analysis:
```
...
Running MIRAI...
Done
Running analysis...
Done
Processing output...
Done
```

Finally, a folder called `output` should be created which will contain all of the analysis
artifacts from this run. Most importantly, it will contain a file `decoded.json` that has
the decoded Datalog output relation results.

For example, if the output relations are `CheckedType(t)` (indicating that a type was checked
as expected) and `NotCheckedType(t)` (indicating that a type that was supposed to be checked
was not), you may see results like this in `decoded.json`:
```
[
    {
        "name": "CheckedType",
        "operands": [
            {
                "name": "t",
                "index": 34,
                "string": "file_format::IdentifierIndex",
                "op_type": "Type"
            }
        ]
    },
    ...
    {
        "name": "NotCheckedType",
        "operands": [
            {
                "name": "t",
                "index": 16,
                "string": "usize",
                "op_type": "Type"
            }
        ]
    }
]
```

In this case, an analyst may focus on the occurrences of `NotCheckedType` as they represent
potential issues in the analyzed crate. If the Soufflé Datalog backend is used, further triage
may be done via the Soufflé explain command, which produces a proof tree for an output fact.

```
$ cd output
$ souffle -t explain ../analyses/souffle.dl
Explain is invoked.
Enter command > explain NotCheckedType(16)
EdgeType(40, 16)
-------------(R1)
  ValidType(16)   !CheckedType(16)
-------------------------------(R1)
        NotCheckedType(16)
```

Also useful is Soufflé's `explainnegation` command which can be used to interactively explore
why a particular fact _does not_ exist.
