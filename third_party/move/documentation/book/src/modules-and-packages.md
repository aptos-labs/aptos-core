# Modules, Packages, and Imports

## Modules and Scripts

Move has two different types of programs: **modules** and **scripts**. Modules are libraries that define struct types along with functions that operate on these types. Struct types define the schema of Move's [global storage](./global-storage.md), and module functions define the rules for updating storage. Modules themselves are also stored in global storage. A script is an executable entrypoint similar to a `main` function in a conventional language. A script typically calls functions of a published module that perform updates to global storage. Scripts are ephemeral code snippets that are not published in global storage.

A Move source file (or **compilation unit**) may contain multiple modules and scripts. However, publishing a module and executing a script are separate VM operations.

### Syntax

#### Scripts

> **Note:** To learn how to publish and execute a Move script, follow the [Move Scripts](https://aptos.dev/build/smart-contracts/scripts/script-tutorial) example.

A script has the following structure:

```text
script {
    <use>*
    <constants>*
    fun <identifier><[type parameters: constraint]*>([identifier: type]*) <function_body>
}
```

A `script` block must start with all of its [`use`](./modules-and-packages.md) declarations, followed by any [constants](./constants.md) and (finally) the main
[function](./functions.md) declaration.
The main function can have any name (i.e., it need not be called `main`), is the only function in a script block, can have any number of
arguments, and must not return a value. Here is an example with each of these components:

```move
script {
    // Import the debug module published at the named account address std.
    use std::debug;

    const ONE: u64 = 1;

    fun main(x: u64) {
        let sum = x + ONE;
        debug::print(&sum)
    }
}
```

Scripts have very limited power—they cannot declare friends or struct types, and they cannot access global storage. Their primary purpose is to invoke module functions.

#### Modules

A module has the following syntax:

```move
module <address>::<identifier> {
    (<use> | <friend> | <type> | <function> | <constant>)*
}
```

where `<address>` is a valid [named or literal address](./primitive-types.md).

For example:

```move
module 0x42::example {
    struct Example has copy, drop { i: u64 }

    use std::debug;
    friend 0x42::another_example;

    const ONE: u64 = 1;

    public fun print(x: u64) {
        let sum = x + ONE;
        let example = Example { i: sum };
        debug::print(&sum)
    }
}
```

The `module 0x42::example` part specifies that the module `example` will be published under the [account address](./primitive-types.md) `0x42` in [global storage](./global-storage.md).

Modules can also be declared using [named addresses](./primitive-types.md). For example:

```move
module example_addr::example {
    struct Example has copy, drop { a: address }

    use std::debug;
    friend example_addr::another_example;

    public fun print() {
        let example = Example { a: @example_addr };
        debug::print(&example)
    }
}
```

Because named addresses only exist at the source language level and during compilation,
named addresses will be fully substituted for their value at the bytecode
level. For example if we had the following code:

```move
script {
    fun example() {
        my_addr::m::foo(@my_addr);
    }
}
```

and we compiled it with `my_addr` set to `0xC0FFEE`, then it would be equivalent
to the following operationally:

```move
script {
    fun example() {
        0xC0FFEE::m::foo(@0xC0FFEE);
    }
}
```

However, at the source level, these _are not equivalent_—the function
`m::foo` _must_ be accessed through the `my_addr` named address, and not through
the numerical value assigned to that address.

Module names can start with letters `a` to `z` or letters `A` to `Z`. After the first character, module names can contain underscores `_`, letters `a` to `z`, letters `A` to `Z`, or digits `0` to `9`.

```move
module my_module {}
module foo_bar_42 {}
```

Typically, module names start with a lowercase letter. A module named `my_module` should be stored in a source file named `my_module.move`.

All elements inside a `module` block can appear in any order.
Fundamentally, a module is a collection of [`types`](./structs-and-enums.md) and [`functions`](./functions.md).
The [`use`](./modules-and-packages.md) keyword is used to import types from other modules.
The [`friend`](./modules-and-packages.md) keyword specifies a list of trusted modules.
The [`const`](./constants.md) keyword defines private constants that can be used in the functions of a module.

## Packages

Packages allow Move programmers to more easily re-use code and share it
across projects. The Move package system allows programmers to easily do the following:

- Define a package containing Move code;
- Parameterize a package by [named addresses](./primitive-types.md);
- Import and use packages in other Move code and instantiate named addresses;
- Build packages and generate associated compilation artifacts from packages; and
- Work with a common interface around compiled Move artifacts.

### Package Layout and Manifest Syntax

A Move package source directory contains a `Move.toml` package manifest
file along with a set of subdirectories:

```
a_move_package/
├── Move.toml
├── sources (required)/
│   ├── module.move
│   └── *.move
├── examples (optional, test & dev mode)/
├── scripts (optional, can also put in sources)/
├── doc_templates (optional)/
└── tests (optional, test mode)/
```

The directories marked `required` _must_ be present in order for the directory
to be considered a Move package and to be compiled. Optional directories can
be present, and if so will be included in the compilation process. Depending on
the mode that the package is built with (`test` or `dev`), the `tests` and
`examples` directories will be included as well.

The `sources` directory can contain both Move modules and Move scripts (both
Move scripts and modules containing script functions). The `examples`
directory can hold additional code to be used only for development and/or
tutorial purposes that will not be included when compiled outside `test` or
`dev` mode.

A `scripts` directory is supported so Move scripts can be separated
from modules if that is desired by the package author. The `scripts`
directory will always be included for compilation if it is present.
Documentation will be built using any documentation templates present in
the `doc_templates` directory.

#### Move.toml

The Move package manifest is defined within the `Move.toml` file and has the
following syntax. Optional fields are marked with `*`, `+` denotes
one or more elements:

```toml filename="Move.toml"
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
<string> = { git = <URL ending in .git>, subdir=<path to dir containing Move.toml inside git repo>, rev=<git commit hash or branch name>, addr_subst* = { (<string> = (<string> | "<hex_address>"))+ } } # git dependencies

[dev-addresses] # (Optional section) Same as [addresses] section, but only included in "dev" and "test" modes
# One or more lines declaring dev named addresses in the following format
<addr_name> = "_" | "<hex_address>" # e.g., std = "_" or my_addr = "0xC0FFEECAFE"

[dev-dependencies] # (Optional section) Same as [dependencies] section, but only included in "dev" and "test" modes
# One or more lines declaring dev dependencies in the following format
<string> = { local = <string>, addr_subst* = { (<string> = (<string> | <address>))+ } }
```

An example of a minimal package manifest:

```toml
[package]
name = "AName"
version = "0.0.0"
```

An example of a more standard package manifest that also includes the Move
standard library and instantiates the named address `Std` from it with the
address value `0x1`:

```toml
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
MoveStdlib = { git = "https://github.com/aptos-labs/aptos-framework", subdir="move-stdlib", rev = "mainnet" }

[dev-addresses] # For use when developing this module
address_to_be_filled_in = "0x101010101"
```

Most of the sections in the package manifest are self-explanatory, but named
addresses can be a bit difficult to understand, so it's worth examining them in
a bit more detail.

### Named Addresses During Compilation

Recall that Move has [named addresses](./primitive-types.md) and that
named addresses cannot be declared in Move. Because of this, until now
named addresses and their values needed to be passed to the compiler on the
command line. With the Move package system this is no longer needed, and
you can declare named addresses in the package, instantiate other named
addresses in scope, and rename named addresses from other packages within
the Move package system manifest file. Let's go through each of these
individually:

#### Declaration

Let's say we have a Move module in `example_pkg/sources/A.move` as follows:

```move
module named_addr::A {
  public fun x(): address { @named_addr }
}
```

We could in `example_pkg/Move.toml` declare the named address `named_addr` in
two different ways. The first:

```toml
[package]
name = "ExamplePkg"
# ...
[addresses]
named_addr = "_"
```

Declares `named_addr` as a named address in the package `ExamplePkg` and
that _this address can be any valid address value_. Therefore, an importing
package can pick the value of the named address `named_addr` to be any address
it wishes. Intuitively you can think of this as parameterizing the package
`ExamplePkg` by the named address `named_addr`, and the package can then be
instantiated later on by an importing package.

`named_addr` can also be declared as:

```toml
[package]
name = "ExamplePkg"
# ...
[addresses]
named_addr = "0xCAFE"
```

which states that the named address `named_addr` is exactly `0xCAFE` and cannot be
changed. This is useful so other importing packages can use this named
address without needing to worry about the exact value assigned to it.

With these two different declaration methods, there are two ways that
information about named addresses can flow in the package graph:

- The former ("unassigned named addresses") allows named address values to flow
  from the importation site to the declaration site.
- The latter ("assigned named addresses") allows named address values to flow
  from the declaration site upwards in the package graph to usage sites.

With these two methods for flowing named address information throughout the
package graph the rules around scoping and renaming become important to
understand.

### Scoping and Renaming of Named Addresses

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

```toml
[package]
name = "P"
# ...
[dependencies]
P1 = { local = "some_path_to_P1", addr_subst = { "P1N" = "N" } }
P2 = { local = "some_path_to_P2"  }
```

With this renaming `N` refers to the `N` from `P2` and `P1N` will refer to `N`
coming from `P1`:

```move
module N::A {
    public fun x(): address { @P1N }
}
```

It is important to note that _renaming is not local_: once a named address `N`
has been renamed to `N2` in a package `P`, all packages that import `P` will not
see `N` but only `N2` unless `N` is reintroduced from outside of `P`. This is
why rule (2) in the scoping rules at the start of this section specifies a
"dependency path in the package graph between `P` and the declaring
package of `N` with no renaming of `N`."

#### Instantiation

Named addresses can be instantiated multiple times across the package graph as
long as it is always with the same value. It is an error if the same named
address (regardless of renaming) is instantiated with differing values across
the package graph.

A Move package can only be compiled if all named addresses resolve to a value.
This presents issues if the package wishes to expose an uninstantiated named
address. This is what the `[dev-addresses]` section solves. This section can
set values for named addresses, but cannot introduce any named addresses.
Additionally, only the `[dev-addresses]` in the root package are included in
`dev` mode. For example, a root package with the following manifest would not compile
outside of `dev` mode since `named_addr` would be uninstantiated:

```toml
[package]
name = "ExamplePkg"
# ...
[addresses]
named_addr = "_"

[dev-addresses]
named_addr = "0xC0FFEE"
```

### Usage, Artifacts, and Data Structures

The Move package system comes with a command line option as part of the Move
CLI `move <flags> <command> <command_flags>`. Unless a
particular path is provided, all package commands will run in the current working
directory. The full list of commands and flags for the Move CLI can be found by
running `move --help`.

#### Usage

A package can be compiled either through the Move CLI commands, or as a library
command in Rust with the function `compile_package`. This will create a
`CompiledPackage` that holds the compiled bytecode along with other compilation
artifacts (source maps, documentation, ABIs) in memory. This `CompiledPackage`
can be converted to an `OnDiskPackage` and vice versa -- the latter being the data of
the `CompiledPackage` laid out in the file system in the following format:

```
a_move_package/
├── .../
└── build/
    ├── dependency_name/
    │   ├── BuildInfo.yaml
    │   ├── bytecode_modules/
    │   │   ├── module_name.mv
    │   │   └── *.mv
    │   ├── source_maps/
    │   │   ├── module_name.mvsm
    │   │   └── *.mvsm
    │   ├── bytecode_scripts/
    │   │   ├── script_name.mv
    │   │   └── *.mv
    │   ├── abis/
    │   │   ├── script_name.abi
    │   │   ├── *.abi
    │   │   └── module_name/
    │   │       ├── function_name.abi
    │   │       └── *.abi
    │   └── sources/
    │       └── module_name.move
    └── dependency_name2 .../
```

See the `move-package` crate for more information on these data structures and
how to use the Move package system as a Rust library.

### Using Bytecode for Dependencies

Move bytecode can be used as dependencies when the Move source code for those dependencies is not available locally. To use this feature, you will need to co-locate the files in directories at the same level and then specify their paths in the corresponding `Move.toml` files.

### Requirements and limitations

Using local bytecode as dependencies requires bytecode files to be downloaded locally, and the actual address for each named address must be specified in either `Move.toml` or through `--named-addresses`.

Note that both the `aptos move prove` and `aptos move test` commands currently do not support bytecode as dependencies.

### Recommended structure

We use an example to illustrate the development flow of using this feature. Suppose we want to compile the package `A`. The package layout is:

```
A/
├── Move.toml
└── sources/
    └── AModule.move
```

`A.move` is defined below, depending on the modules `Bar` and `Foo`:

```move filename="A/AModule.move"
module A::AModule {
    use B::Bar;
    use C::Foo;
    public fun foo(): u64 {
        Bar::foo() + Foo::bar()
    }
}
```

Suppose the source of `Bar` and `Foo` are not available but the corresponding bytecode `Bar.mv` and `Foo.mv` are available locally. To use them as dependencies, we would:

Specify `Move.toml` for `Bar` and `Foo`. Note that named addresses are already instantiated with the actual address in the bytecode. In our example, the actual address for `C` is already bound to `0x3`. As a result, the `[addresses]` section must specify `C` as `0x3`, as shown below:

```toml filename="workspace/C/Move.toml"
[package]
name = "Foo"
version = "0.0.0"

[addresses]
C = "0x3"
```

Place the bytecode file and the corresponding `Move.toml` file in the same directory with the bytecode in a `build` subdirectory. Note an empty `sources` directory is **required**. For instance, the layout of the folder `B` (for the package `Bar`) and `C` (for the package `Foo`) would resemble:

```
workspace/
├── A/
│   ├── Move.toml
│   └── sources/
│       └── AModule.move
├── B/
│   ├── Move.toml
│   ├── sources/
│   └── build/
│       └── Bar.mv
└── C/
    ├── Move.toml
    ├── sources/
    └── build/
        └── Foo/
            └── bytecode_modules/
                └── Foo.mv
```

Specify `[dependencies]` in the `Move.toml` of the target (first) package with the location of the dependent (secondary) packages. For instance, assuming all three package directories are at the same level, `Move.toml` of `A` would resemble:

```toml filename="workspace/A/Move.toml"
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

### Overriding the Standard Libraries

When working with third-party packages, you might encounter issues where different versions of the Move and Aptos standard library packages are referenced.

This can lead to package resolution failures.

```
"Error": "Move compilation failed:
  Unable to resolve packages for package 'C':
    While resolving dependency 'B' in package 'C':
      Unable to resolve package dependency 'B':
        While resolving dependency 'AptosFramework' in package 'B':
          Unable to resolve package dependency 'AptosFramework':
            Conflicting dependencies found: package 'AptosFramework' conflicts with 'AptosFramework'
```

To resolve this, you can override the standard library packages using a command-line option. This allows you to enforce a specific version of the standard libraries across your entire dependency tree.

You can apply the override to commands like `aptos move compile`, `aptos move run`, and others. Here is the syntax:

```
--override-std <network name>
```

Where `network_name` can be one of the following:

- devnet
- testnet
- mainnet

## Package Upgrades

Move code (e.g., Move modules) on the Aptos blockchain can be upgraded. This
allows code owners and module developers to update and evolve their contracts
under a single, stable, well-known account address that doesn't change. If a
module upgrade happens, all consumers of that module will automatically receive
the latest version of the code (e.g., the next time they interact with it).

The Aptos blockchain natively supports different _upgrade policies_, which allow
Move developers to explicitly define the constraints around how their Move code
can be upgraded. The default policy is _backwards compatible_. This means that
code upgrades are accepted only if they guarantee that no existing resource storage
or public APIs are broken by the upgrade (including public functions).
This compatibility checking is possible because of Move's strongly typed bytecode
semantics.

We note, however, that even compatible upgrades can have hazardous effects on
applications and dependent Move code (for example, if the semantics of the underlying
module are modified). As a result, developers should be careful when depending on
third-party Move code that can be upgraded on-chain. See
[Security considerations for dependencies](#security-considerations-for-dependencies)
for more details.

### How it works

Move code upgrades on the Aptos blockchain happen at the [Move package](./modules-and-packages.md)
granularity. A package specifies an upgrade policy in the `Move.toml` manifest:

```toml
[package]
name = "MyApp"
version = "0.0.1"
upgrade_policy = "compatible"
...
```

> **Note:** Aptos checks compatibility at the time a Move package is published via an Aptos transaction. This transaction will abort if deemed incompatible.

### How to upgrade

To upgrade already published Move code, simply attempt to republish the code at
the same address where it was previously published. This can be done by following the
instructions for code compilation and publishing using the
[Aptos CLI](https://aptos.dev/build/cli/working-with-move-contracts). For an example,
see the [Your First Move Module](https://aptos.dev/build/guides/first-move-module) tutorial.

### Upgrade policies

There are two different upgrade policies currently supported by Aptos:

- `compatible`: these upgrades must be backwards compatible, specifically:
  - For storage, all old struct declarations must be the same in
    the new code. This ensures that the existing state of storage is
    correctly interpreted by the new code. However, new struct declarations
    can be added.
  - For APIs, all existing public functions must have the same signature as
    before. New functions, including public and entry functions, can be added.
- `immutable`: the code is not upgradeable and is guaranteed to stay the same
  forever.

Those policies are ordered by strength such that `compatible < immutable`,
i.e., compatible is weaker than immutable. The policy of a package on-chain can
only get stronger, not weaker. Moreover, the policy of all dependencies of a
package must be stronger or equal to the policy of the given package. For example,
an `immutable` package cannot refer directly or indirectly to a `compatible` package.
This gives users the guarantee that no unexpected updates can happen under the hood.

Note that there is one exception to the above rule: framework packages
installed at addresses `0x1` to `0xa` are exempted from the dependency check.
This is necessary so one can define an `immutable` package based on the standard
libraries, which have the `compatible` policy to allow critical upgrades and fixes.

### Compatibility rules

When using `compatible` upgrade policy, a module package can be upgraded. However, updates to existing modules already
published previously need to be compatible and follow the rules below:

- All existing structs' fields cannot be updated. This means no new fields can be added and existing fields cannot be
  modified.
- All public and entry functions cannot change their signature (argument types, type arguments, return types). However,
  argument names can change.
- `public(friend)` functions are treated as private and thus their signature can arbitrarily change. This is safe as
  only modules in the same package can call friend functions anyway, and they need to be updated if the signature changes.
- [Enum type upgrade compatibility rules](./structs-and-enums.md#enum-type-upgrade-compatibility).
- Existing abilities on a struct/enum type cannot be removed (but abilities can be added).

When updating your modules, if you see an incompatible error, make sure to check the above rules and fix any violations.

### Security considerations for dependencies

As mentioned above, even compatible upgrades can have disastrous effects for
applications that depend on the upgraded code. These effects can come from bugs,
but they can also be the result of malicious upgrades. For example,
an upgraded dependency can suddenly make all functions abort, breaking the
operation of your Move code. Alternatively, an upgraded dependency can make
all functions suddenly cost much more gas to execute than before the upgrade.
As a result, dependencies on upgradeable packages need to be handled with care:

- The safest dependency is, of course, an `immutable` package. This guarantees
  that the dependency will never change, including its transitive dependencies.
  In order to update an immutable package, the owner would have to introduce a
  new major version, which is practically like deploying a new, separate
  and independent package. This is because major versioning can be expressed
  only by name (e.g., `module feature_v1` and `module feature_v2`). However,
  not all package owners like to publish their code as `immutable`, because this
  takes away the ability to fix bugs and update the code in place.
- If you have a dependency on a `compatible` package, it is highly
  recommended you know and understand the entity publishing the package.
  The highest level of assurance is when the package is governed by a
  Decentralized Autonomous Organization (DAO) where no single user can initiate
  an upgrade; a vote or similar has to be taken. This is the case for the Aptos
  framework.

### Programmatic upgrade

In general, Aptos offers, via the Move module `aptos_framework::code`,
ways to publish code from anywhere in your smart contracts. However,
notice that code published in the current transaction can be executed
only after that transaction ends.

The Aptos framework itself, including all the on-chain administration logic, is
an example of a programmatic upgrade. The framework is marked as `compatible`.
Upgrades happen via specific generated governance scripts. For more details,
see [Aptos Governance](https://aptos.dev/network/blockchain/governance).

## Uses and Aliases

The `use` syntax can be used to create aliases to members in other modules. `use` can be used to
create aliases that last either for the entire module, or for a given expression block scope.

### Syntax

There are several different syntax cases for `use`. Starting with the most simple, we have the
following for creating aliases to other modules

```move
use <address>::<module name>;
use <address>::<module name> as <module alias name>;
```

For example

```move
script {
  use std::vector;
  use std::vector as V;
}
```

`use std::vector;` introduces an alias `vector` for `std::vector`. This means that anywhere you
would want to use the module name `std::vector` (assuming this `use` is in scope), you could use
`vector` instead. `use std::vector;` is equivalent to `use std::vector as vector;`

Similarly `use std::vector as V;` would let you use `V` instead of `std::vector`

```move
module 0x42::example {
  use std::vector;
  use std::vector as V;

  fun new_vecs(): (vector<u8>, vector<u8>, vector<u8>) {
    let v1 = std::vector::empty();
    let v2 = vector::empty();
    let v3 = V::empty();
    (v1, v2, v3)
  }
}
```

If you want to import a specific module member (such as a function, struct, or constant), you can
use the following syntax.

```move
use <address>::<module name>::<module member>;
use <address>::<module name>::<module member> as <member alias>;
```

For example

```move
script {
  use std::vector::empty;
  use std::vector::empty as empty_vec;
}
```

This would let you use the function `std::vector::empty` without full qualification. Instead, you
could use `empty` and `empty_vec` respectively. Again, `use std::vector::empty;` is equivalent to
`use std::vector::empty as empty;`

```move
module 0x42::example {
  use std::vector::empty;
  use std::vector::empty as empty_vec;

  fun new_vecs(): (vector<u8>, vector<u8>, vector<u8>) {
    let v1 = std::vector::empty();
    let v2 = empty();
    let v3 = empty_vec();
    (v1, v2, v3)
  }
}
```

If you want to add aliases for multiple module members at once, you can do so with the following
syntax

```move
use <address>::<module name>::{<module member>, <module member> as <member alias> ... };
```

For example

```move
module 0x42::example {
  use std::vector::{push_back, length as len, pop_back};

  fun swap_last_two<T>(v: &mut vector<T>) {
    assert!(len(v) >= 2, 42);
    let last = pop_back(v);
    let second_to_last = pop_back(v);
    push_back(v, last);
    push_back(v, second_to_last)
  }
}
```

If you need to add an alias to the module itself in addition to module members, you can do that in a
single `use` using `Self`. `Self` is a member of sorts that refers to the module.

```move
script {
  use std::vector::{Self, empty};
}
```

For clarity, all the following are equivalent:

```move
script {
  use std::vector;
  use std::vector as vector;
  use std::vector::Self;
  use std::vector::Self as vector;
  use std::vector::{Self};
  use std::vector::{Self as vector};
}
```

If needed, you can have as many aliases for any item as you like

```move
module 0x42::example {
  use std::vector::{
    Self,
    Self as V,
    length,
    length as len,
  };

  fun pop_twice<T>(v: &mut vector<T>): (T, T) {
    // all options available given the `use` above
    assert!(vector::length(v) > 1, 42);
    assert!(V::length(v) > 1, 42);
    assert!(length(v) > 1, 42);
    assert!(len(v) > 1, 42);

    (vector::pop_back(v), vector::pop_back(v))
  }
}
```

### Inside a `module`

Inside a `module` all `use` declarations are usable regardless of the order of declaration.

```move
module 0x42::example {
  use std::vector;

  fun example(): vector<u8> {
    let v = empty();
    vector::push_back(&mut v, 0);
    vector::push_back(&mut v, 10);
    v
  }

  use std::vector::empty;
}
```

The aliases declared by `use` in the module are usable within that module.

Additionally, the aliases introduced cannot conflict with other module members. See
[Uniqueness](#uniqueness) for more details

### Inside an expression

You can add `use` declarations to the beginning of any expression block

```move
module 0x42::example {

  fun example(): vector<u8> {
    use std::vector::{empty, push_back};

    let v = empty();
    push_back(&mut v, 0);
    push_back(&mut v, 10);
    v
  }
}
```

As with `let`, the aliases introduced by `use` in an expression block are removed at the end of that
block.

```move
module 0x42::example {

  fun example(): vector<u8> {
    let result = {
      use std::vector::{empty, push_back};
      let v = empty();
      push_back(&mut v, 0);
      push_back(&mut v, 10);
      v
    };
    result
  }
}
```

Attempting to use the alias after the block ends will result in an error

```move
module 0x42::example {
  fun example(): vector<u8> {
    let result = {
      use std::vector::{empty, push_back};
      let v = empty();
      push_back(&mut v, 0);
      push_back(&mut v, 10);
      v
    };
    let v2 = empty(); // ERROR!
//           ^^^^^ unbound function 'empty'
    result
  }
}
```

Any `use` must be the first item in the block. If the `use` comes after any expression or `let`, it
will result in a parsing error

```move
script {
  fun example() {
    {
      let x = 0;
      use std::vector; // ERROR!
      let v = vector::empty();
    }
  }
}

```

### Naming rules

Aliases must follow the same rules as other module members. This means that aliases to structs or
constants must start with `A` to `Z`

```move
address 0x42 {
  module data {
    struct S {}
    const FLAG: bool = false;
    fun foo() {}
  }
  module example {
    use 0x42::data::{
      S as s, // ERROR!
      FLAG as fLAG, // ERROR!
      foo as FOO,  // valid
      foo as bar, // valid
    };
  }
}
```

### Uniqueness

Inside a given scope, all aliases introduced by `use` declarations must be unique.

For a module, this means aliases introduced by `use` cannot overlap

```move
module 0x42::example {
  use std::vector::{empty as foo, length as foo}; // ERROR!
  //                                        ^^^ duplicate 'foo'

  use std::vector::empty as bar;
  use std::vector::length as bar; // ERROR!
  //                         ^^^ duplicate 'bar'
}
```

And, they cannot overlap with any of the module's other members

```move
address 0x42 {
  module data {
    struct S {}
  }
  module example {
    use 0x42::data::S;

    struct S { value: u64 } // ERROR!
    //     ^ conflicts with alias 'S' above
  }
}
```

Inside an expression block, they cannot overlap with each other, but they can
[shadow](#shadowing) other aliases or names from an outer scope

### Shadowing

`use` aliases inside of an expression block can shadow names (module members or aliases) from the
outer scope. As with shadowing of locals, the shadowing ends at the end of the expression block;

```move
module 0x42::example {

  struct WrappedVector { vec: vector<u64> }

  fun empty(): WrappedVector {
    WrappedVector { vec: std::vector::empty() }
  }

  fun example1(): (WrappedVector, WrappedVector) {
    let vec = {
      use std::vector::{empty, push_back};
      // 'empty' now refers to std::vector::empty

      let v = empty();
      push_back(&mut v, 0);
      push_back(&mut v, 1);
      push_back(&mut v, 10);
      v
    };
    // 'empty' now refers to Self::empty

    (empty(), WrappedVector { vec })
  }

  fun example2(): (WrappedVector, WrappedVector) {
    use std::vector::{empty, push_back};
    let w: WrappedVector = {
      use 0x42::example::empty;
      empty()
    };
    push_back(&mut w.vec, 0);
    push_back(&mut w.vec, 1);
    push_back(&mut w.vec, 10);

    let vec = empty();
    push_back(&mut vec, 0);
    push_back(&mut vec, 1);
    push_back(&mut vec, 10);

    (w, WrappedVector { vec })
  }
}
```

### Unused Use or Alias

An unused `use` will result in an error

```move
module 0x42::example {
  use std::vector::{empty, push_back}; // ERROR!
  //                       ^^^^^^^^^ unused alias 'push_back'

  fun example(): vector<u8> {
    empty()
  }
}
```

## Friends

The `friend` syntax is used to declare modules that are trusted by the current module.
A trusted module is allowed to call any function defined in the current module that has the `public(friend)` visibility.
For details on function visibilities, please refer to the _Visibility_ section in [Functions](./functions.md).

### Friend declaration

A module can declare other modules as friends via friend declaration statements, in the format of

- `friend <address::name>` — friend declaration using fully qualified module name like the example below, or

  ```move
  module 0x42::a {
      friend 0x42::b;
  }
  ```

- `friend <module-name-alias>` — friend declaration using a module name alias, where the module alias is introduced via the `use` statement.

  ```move
  module 0x42::a {
      use 0x42::b;
      friend b;
  }
  ```

A module may have multiple friend declarations, and the union of all the friend modules forms the friend list.
In the example below, both `0x42::B` and `0x42::C` are considered as friends of `0x42::A`.

```move
module 0x42::a {
    friend 0x42::b;
    friend 0x42::c;
}
```

Unlike `use` statements, `friend` can only be declared in the module scope and not in the expression block scope.
`friend` declarations may be located anywhere a top-level construct (e.g., `use`, `function`, `struct`, etc.) is allowed.
However, for readability, it is advised to place friend declarations near the beginning of the module definition.

Note that the concept of friendship does not apply to Move scripts:

- A Move script cannot declare `friend` modules as doing so is considered meaningless: there is no mechanism to call the function defined in a script.
- A Move module cannot declare `friend` scripts either, because scripts are ephemeral code snippets that are never published to global storage.

#### Friend declaration rules

Friend declarations are subject to the following rules:

- A module cannot declare itself as a friend.

  ```move
  module 0x42::m {
    friend Self; // ERROR!
  //       ^^^^ Cannot declare the module itself as a friend
  }

  module 0x43::m {
    friend 0x43::M; // ERROR
  //       ^^^^^^^ Cannot declare the module itself as a friend
  }
  ```

- Friend modules must be known by the compiler

  ```move
  module 0x42::m {
    friend 0x42::nonexistent; // ERROR!
    //     ^^^^^^^^^^^^^^^^^ Unbound module '0x42::nonexistent'
  }
  ```

- Friend modules must be within the same account address. (Note: this is not a technical requirement but rather a policy decision which _may_ be relaxed later.)

  ```move
  module 0x42::m {}

  module 0x43::n {
    friend 0x42::m; // ERROR!
  //       ^^^^^^^ Cannot declare modules out of the current address as a friend
  }
  ```

- Friend relationships cannot create cyclic module dependencies.

  Cycles are not allowed in the friend relationships, e.g., the relation `0x2::a` friends `0x2::b` friends `0x2::c` friends `0x2::a` is not allowed.
  More generally, declaring a friend module adds a dependency upon the current module to the friend module (because the purpose is for the friend to call functions in the current module).
  If that friend module is already used, either directly or transitively, a cycle of dependencies would be created.

  ```move
  address 0x2 {
    module a {
      use 0x2::c;
      friend 0x2::b;

      public fun a() {
        c::c()
      }
    }

    module b {
      friend 0x2::c; // ERROR!
    //       ^^^^^^ This friend relationship creates a dependency cycle: '0x2::b' is a friend of '0x2::a' uses '0x2::c' is a friend of '0x2::b'
    }

    module c {
      public fun c() {}
    }
  }
  ```

- The friend list for a module cannot contain duplicates.

  ```move
  address 0x42 {
    module a {}

    module m {
      use 0x42::a as aliased_a;
      friend 0x42::A;
      friend aliased_a; // ERROR!
    //       ^^^^^^^^^ Duplicate friend declaration '0x42::a'. Friend declarations in a module must be unique
    }
  }
  ```
