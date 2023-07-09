# Packages

Packages allow Move programmers to more easily re-use code and share it
across projects. The Move package system allows programmers to easily:
* Define a package containing Move code;
* Parameterize a package by [named addresses](./address.md);
* Import and use packages in other Move code and instantiate named addresses;
* Build packages and generate associated compilation artifacts from packages; and
* Work with a common interface around compiled Move artifacts.

## Package Layout and Manifest Syntax

A Move package source directory contains a `Move.toml` package manifest
file along with a set of subdirectories:

```
a_move_package
├── Move.toml      (required)
├── sources        (required)
├── examples       (optional, test & dev mode)
├── scripts        (optional)
├── doc_templates  (optional)
└── tests          (optional, test mode)
```

The directories marked `required` _must_ be present in order for the directory
to be considered a Move package and to be compiled. Optional directories can
be present, and if so will be included in the compilation process. Depending on
the mode that the package is built with (`test` or `dev`), the `tests` and
`examples` directories will be included as well.

The `sources` directory can contain both Move modules and Move scripts (both
transaction scripts and modules containing script functions). The `examples`
directory can hold additional code to be used only for development and/or
tutorial purposes that will not be included when compiled outside `test` or
`dev` mode.

A `scripts` directory is supported so transaction scripts can be separated
from modules if that is desired by the package author. The `scripts`
directory will always be included for compilation if it is present.
Documentation will be built using any documentation templates present in
the `doc_templates` directory.

### Move.toml

The Move package manifest is defined within the `Move.toml` file and has the
following syntax. Optional fields are marked with `*`, `+` denotes
one or more elements:

```
[package]
name = <string>                  # e.g., "MoveStdlib"
version = "<uint>.<uint>.<uint>" # e.g., "0.1.1"
license* = <string>              # e.g., "MIT", "GPL", "Apache 2.0"
authors* = [<string>]            # e.g., ["Joe Smith (joesmith@noemail.com)", "Jane Smith (janesmith@noemail.com)"]

[addresses]  # (Optional section) Declares named addresses in this package and instantiates named addresses in the package graph
# One or more lines declaring named addresses in the following format
<addr_name> = "_" | "<hex_address>" # e.g., std = "_" or my_addr = "0xC0FFEECAFE"

[dependencies] # (Optional section) Paths to dependencies and instantiations or renamings of named addresses from each dependency
# One or more lines declaring dependencies in the following format
<string> = { local = <string>, addr_subst* = { (<string> = (<string> | "<hex_address>"))+ } } # local dependencies
<string> = { git = <URL ending in .git>, subdir=<path to dir containing Move.toml inside git repo>, rev=<git commit hash>, addr_subst* = { (<string> = (<string> | "<hex_address>"))+ } } # git dependencies

[dev-addresses] # (Optional section) Same as [addresses] section, but only included in "dev" and "test" modes
# One or more lines declaring dev named addresses in the following format
<addr_name> = "_" | "<hex_address>" # e.g., std = "_" or my_addr = "0xC0FFEECAFE"

[dev-dependencies] # (Optional section) Same as [dependencies] section, but only included in "dev" and "test" modes
# One or more lines declaring dev dependencies in the following format
<string> = { local = <string>, addr_subst* = { (<string> = (<string> | <address>))+ } }
```

An example of a minimal package manifest with one local dependency and one git dependency:

```
[package]
name = "AName"
version = "0.0.0"
```

An example of a more standard package manifest that also includes the Move
standard library and instantiates the named address `Std` from it with the
address value `0x1`:

```
[package]
name = "AName"
version = "0.0.0"
license = "Apache 2.0"

[addresses]
address_to_be_filled_in = "_"
specified_address = "0xB0B"

[dependencies]
# Local dependency
LocalDep = { local = "projects/move-awesomeness", addr_subst = { "std" = "0x1" } }
# Git dependency
MoveStdlib = { git = "https://github.com/diem/diem.git", subdir="language/move-stdlib", rev = "56ab033cc403b489e891424a629e76f643d4fb6b" }

[dev-addresses] # For use when developing this module
address_to_be_filled_in = "0x101010101"
```

Most of the sections in the package manifest are self explanatory, but named
addresses can be a bit difficult to understand so it's worth examining them in
a bit more detail.

## Named Addresses During Compilation

Recall that Move has [named addresses](./address.md) and that
named addresses cannot be declared in Move. Because of this, until now
named addresses and their values needed to be passed to the compiler on the
command line. With the Move package system this is no longer needed, and
you can declare named addresses in the package, instantiate other named
addresses in scope, and rename named addresses from other packages within
the Move package system manifest file. Let's go through each of these
individually:

### Declaration

Let's say we have a Move module in `example_pkg/sources/A.move` as follows:

```move
module named_addr::A {
    public fun x(): address { @named_addr }
}
```

We could in `example_pkg/Move.toml` declare the named address `named_addr` in
two different ways. The first:

```
[package]
name = "ExamplePkg"
...
[addresses]
named_addr = "_"
```

Declares `named_addr` as a named address in the package `ExamplePkg` and
that _this address can be any valid address value_. Therefore an importing
package can pick the value of the named address `named_addr` to be any address
it wishes. Intuitively you can think of this as parameterizing the package
`ExamplePkg` by the named address `named_addr`, and the package can then be
instantiated later on by an importing package.

`named_addr` can also be declared as:

```
[package]
name = "ExamplePkg"
...
[addresses]
named_addr = "0xCAFE"
```

which states that the named address `named_addr` is exactly `0xCAFE` and cannot be
changed. This is useful so other importing packages can use this named
address without needing to worry about the exact value assigned to it.

With these two different declaration methods, there are two ways that
information about named addresses can flow in the package graph:
* The former ("unassigned named addresses") allows named address values to flow
  from the importation site to the declaration site.
* The latter ("assigned named addresses") allows named address values to flow
  from the declaration site upwards in the package graph to usage sites.

With these two methods for flowing named address information throughout the
package graph the rules around scoping and renaming become important to
understand.

## Scoping and Renaming of Named Addresses

A named address `N` in a package `P` is in scope if:
1. It declares a named address `N`; or
2. A package in one of `P`'s transitive dependencies declares the named address
  `N` and there is a dependency path in the package graph between `P` and the
  declaring package of `N` with no renaming of `N`.

Additionally, every named address in a package is exported. Because of this and
the above scoping rules each package can be viewed as coming with a set of
named addresses that will be brought into scope when the package is imported,
e.g., if the `ExamplePkg` package was imported, that importation would bring
into scope the `named_addr` named address. Because of this, if `P` imports two
packages `P1` and `P2` both of which declare a named address `N` an issue
arises in `P`: which "`N`" is meant when `N` is referred to in `P`? The one
from `P1` or `P2`? To prevent this ambiguity around which package a named
address is coming from, we enforce that the sets of scopes introduced by all
dependencies in a package are disjoint, and provide a way to _rename named
addresses_ when the package that brings them into scope is imported.

Renaming a named address when importing can be done as follows in our `P`,
`P1`, and `P2` example above:

```
[package]
name = "P"
...
[dependencies]
P1 = { local = "some_path_to_P1", addr_subst = { "P1N" = "N" } }
P2 = { local = "some_path_to_P2"  }
```

With this renaming `N` refers to the `N` from `P2` and `P1N` will refer to `N`
coming from `P1`:

```
module N::A {
    public fun x(): address { @P1N }
}
```

It is important to note that _renaming is not local_: once a named address `N`
has been renamed to `N2` in a package `P` all packages that import `P` will not
see `N` but only `N2` unless `N` is reintroduced from outside of `P`. This is
why rule (2) in the scoping rules at the start of this section specifies a
"dependency path in the package graph between `P` and the declaring
package of `N` with no renaming of `N`."

### Instantiation

Named addresses can be instantiated multiple times across the package graph as
long as it is always with the same value. It is an error if the same named
address (regardless of renaming) is instantiated with differing values across
the package graph.

A Move package can only be compiled if all named addresses resolve to a value.
This presents issues if the package wishes to expose an uninstantiated named
address. This is what the `[dev-addresses]` section solves. This section can
set values for named addresses, but cannot introduce any named addresses.
Additionally, only the `[dev-addresses]` in the root package are included in
`dev` mode. For example a root package with the following manifest would not compile
outside of `dev` mode since `named_addr` would be uninstantiated:

```
[package]
name = "ExamplePkg"
...
[addresses]
named_addr = "_"

[dev-addresses]
named_addr = "0xC0FFEE"
```

## Usage, Artifacts, and Data Structures

The Move package system comes with a command line option as part of the Move
CLI `move <flags> <command> <command_flags>`. Unless a
particular path is provided, all package commands will run in the current working
directory. The full list of commands and flags for the Move CLI can be found by
running `move --help`.

### Usage

A package can be compiled either through the Move CLI commands, or as a library
command in Rust with the function `compile_package`. This will create a
`CompiledPackage` that holds the compiled bytecode along with other compilation
artifacts (source maps, documentation, ABIs) in memory. This `CompiledPackage`
can be converted to an `OnDiskPackage` and vice versa -- the latter being the data of
the `CompiledPackage` laid out in the file system in the following format:

```
a_move_package
├── Move.toml
...
└── build
    ├── <dep_pkg_name>
    │   ├── BuildInfo.yaml
    │   ├── bytecode_modules
    │   │   └── *.mv
    │   ├── source_maps
    │   │   └── *.mvsm
    │   ├── bytecode_scripts
    │   │   └── *.mv
    │   ├── abis
    │   │   ├── *.abi
    │   │   └── <module_name>/*.abi
    │   └── sources
    │       └── *.move
    ...
    └── <dep_pkg_name>
        ├── BuildInfo.yaml
        ...
        └── sources
```

See the `move-package` crate for more information on these data structures and
how to use the Move package system as a Rust library.

## Using Bytecode for Dependencies

Move bytecode can be used as dependencies when the Move source code for those dependencies are not available locally. To use this feature, you will need co-locate the files in directories at the same level and then specify their paths in the corresponding `Move.toml` files.

## Requirements and limitations

Using local bytecode as dependencies requires bytecode files to be downloaded locally, and the actual address for each named address must be specified in either `Move.toml` or through `--named-addresses`.

Note, both `aptos move prove` and `aptos move test` commands, currently, do not support bytecode as dependencies.

## Recommended structure

We use an example to illustrate the dev flow of using this feature. Suppose we want to compile the package `A`. The package layout is:
```rust
./A
├── Move.toml
├── sources
  ├ AModule.move
```

`A.move` is defined below, depending on the modules `Bar` and `Foo`:

```rust
module A::AModule {
    use B::Bar;
    use C::Foo;
    public fun foo(): u64 {
        Bar::foo() + Foo::bar()
    }
}
```

Suppose the source of `Bar` and `Foo` are not available but the corresponding bytecode `Bar.mv` and `Foo.mv` are available locally. To use them as dependencies, we would:

Specify `Move.toml` for `Bar` and `Foo`. Note that named addresses are already instantiated with the actual address in the bytecode. In our example, the actual address for `C` is already bound to `0x3`. As a result, `[addresses]` must be specified `C` as `0x3`, as shown below:

```rust
[package]
name = "Foo"
version = "0.0.0"
[addresses]
C = "0x3"
```

Place the bytecode file and the corresponding `Move.toml` file in the same directory with the bytecode in a `build` subdirectory. Note an empty `sources` directory is **required**. For instance, the layout of the folder `B` (for the package `Bar`) and `C` (for the package `Foo`) would resemble:

```rust
./B
├── Move.toml
├── sources
├── build
 ├ Bar.mv
```

```rust
./C
├── Move.toml
├── sources
├── build
  ├── Foo
   ├──bytecode_modules
     ├ Foo.mv
```

Specify `[dependencies]` in the `Move.toml` of the target (first) package with the location of the dependent (secondary) packages. For instance, assuming all three package directories are at the same level, `Move.toml` of `A` would resemble:

```rust
[package]
name = "A"
version = "0.0.0"
[addresses]
A = "0x2"
[dependencies]
Bar = { local = "../B" }
Foo = { local = "../C" }
```

Note that if both the bytecode and the source code of the same package exist in the search paths, the compiler will complain that the declaration is duplicated.
