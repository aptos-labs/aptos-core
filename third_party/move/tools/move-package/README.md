---
id: move-package
title: Move Package
custom_edit_url: https://github.com/move-language/move/edit/main/language/tools/move-package/README.md
---

# Summary

The Move package crate contains the logic for parsing, resolving, and
building Move packages. It is meant to be used as a library, both for
building packages (e.g., by the Move CLI), or by other applications that
may be working with Move packages. The package system is split into three
main phases: parsing, resolution, and compilation.

## Parsing and Manifest Layout

The parsing and manifest layout logic is defined in the
[`./src/source_package`](./src/source_package) directory. This defines the
layout and the set of required and optional directories for a Move package
in [`./src/source_package/layout.rs`](./src/source_package/layout.rs), it
defines the format of the parsed Move package manifest ("`Move.toml`") in
[`./src/source_package/parsed_manifest.rs`](./src/source_package/parsed_manifest.rs),
and it defines the parser for the Move package manifest in
[`./src/source_package/manifest_parser.rs`](./src/source_package/manifest_parser.rs).
Note that we don't have a tokenizer/lexer as we use the TOML lexer. This
also resolves git dependencies to where they will live on the local file
system (but does not clone them).

## Resolution

The resolution phase is responsible for resolving all packages and building
the package graph which represents the dependency relations between
packages. It is also responsible for ensuring that all named addresses have
a value assigned to them, and that there are no conflicting assignments.
The package graph is rooted at the package being built and is a DAG.

When building the package graph we do the following conceptual operations:
verify that dependencies exist at the declared locations, that their
package names and source digests match (if applicable), clone git
dependencies if they don't already exist locally, build a dependency graph
of Move packages and ensure this forms a DAG, compute an
assignment for each named address in each Move package in the package
graph, and ensure that the resulting named address assignment is valid.

All of the above steps are fairly straightforward, with the possible
exception of named addresses: each package will have a set of in-scope
named addresses. The set of in-scope named addresses for a package `P` is
defined as the transitive closure of all named addresses in the
dependencies of `P`. Additionally, a package can rename named addresses
that are in-scope as long as the final assignment of a value to the set of
named addresses can be unified. To ensure that named addresses are
unifiable across renamings, resolution performs unification across named
addresses using a `Rc<RefCell<Opton<Address>>>`: when a named address first
enters scope in the package graph a `Rc<RefCell<..>>` is created for it.
This refcell is then shared to all uses of the named address _even across
renamings_ and when a value is assigned to it, the value must (1) match the
current value contained within the `Option`, or (2) the `Option` is `None`,
and the value is placed into the refcell.

## Compilation

The final stage of the package system is compilation. All logic relating to
the final build artifacts, or global environment creation, is contained in
the [`./src/compilation`](./src/compilation) directory. The package layout
for compiled Move packages is defined in
[`./src/compilation/package_layout.rs`](./src/compilation/package_layout.rs).

The [`./src/compilation/build_plan.rs`](./src/compilation/build_plan.rs)
contains the logic for driving the compilation of a package and the
compilation of all of the package's dependencies given a valid resolution
graph. The logic in
[`./src/compilation/compiled_package.rs`](./src/compilation/compiled_package.rs)
contains the definition of the in-memory representation of compiled Move
packages and other data structures and APIs relating to compiled Move
packages, along with the logic for compiling a _single_ Move package
assuming all of its dependencies are already compiled and saved to disk.
This is driven by the logic in
[`./src/compilation/build_plan.rs`](./src/compilation/build_plan.rs). The
compilation process is also responsible for generating documentation, ABIs
and the like, along with determining if a cached version of the
to-be-built package already exists and if so, if the cached version
can be used or if the cached copy is invalid and needs to be recompiled.

One important thing to note here is that depending on the compilation
flags, the caching policy may need to be updated and the `compiler_driver`
function that is passed into the compilation process may change. However,
what this function should be is determined by the client of the Move
package library. In particular, when testing even if we are recompiling
with the same flags we cannot cache the root package as we need to compile
it to generate the test plan that will be used by the unit test runner
later on. This gathering of the test plan is inserted into the compilation
process via the `compiler_driver` function that is passed in by the client.
In this case, the [`../move-cli/src/package`](../move-cli/src/package) is
the client and it is responsible for supplying the correct function as the
compiler driver to collect the test plan and to later pass that to the unit
test runner.
