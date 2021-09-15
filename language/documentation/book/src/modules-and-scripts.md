# Modules and Scripts

Move has two different types of programs: ***Modules*** and ***Scripts***. Modules are libraries that define struct types along with functions that operate on these types. Struct types define the schema of Move's [global storage](./global-storage-structure.md), and module functions define the rules for updating storage. Modules themselves are also stored in global storage. Scripts are executable entrypoints similar to a `main` function in a conventional language. A script typically calls functions of a published module that perform updates to global storage. Scripts are ephemeral code snippets that are not published in global storage.

A Move source file (or **compilation unit**) may contain multiple modules and scripts. However, publishing a module or executing a script are separate VM operations.

## Syntax

### Scripts

A script has the following structure:

```text
script {
    <use>*
    <constants>*
    fun <identifier><[type parameters: constraint]*>([identifier: type]*) <function_body>
}
```

A `script` block must start with all of its [use](./uses.md) declarations, followed by any [constants](./constants.md) and (finally) the main
[function](./functions.md) declaration.
The main function can have any name (i.e., it need not be called `main`), is the only function in a script block, can have any number of
arguments, and must not return a value. Here is an example with each of these components:

```move
script {
    // Import the Debug module published at the named account address Std.
    use Std::Debug;

    const ONE: u64 = 1;

    fun main(x: u64) {
        let sum = x + ONE;
        Debug::print(&sum)
    }
}
```

Scripts have very limited power—they cannot declare friends, struct types or access global storage. Their primary purpose is to invoke module functions.

### Modules

A Module has the following syntax:

```text
module <address>::<identifier> {
    (<use> | <friend> | <type> | <function> | <constant>)*
}
```

where `<address>` is a valid [named or literal address](./address.md).

For example:

```move
module 0x42::Test {
    struct Example has copy, drop { i: u64 }

    use Std::Debug;
    friend 0x42::AnotherTest;

    const ONE: u64 = 1;

    public fun print(x: u64) {
        let sum = x + ONE;
        let example = Example { i: sum };
        Debug::print(&sum)
    }
}
```

The `module 0x42::Test` part specifies that the module `Test` will be published under the [account address](./address.md) `0x42` in [global storage](./global-storage-structure.md).

Modules can also be declared using [named addresses](./address.md). For example:

```move
module TestAddr::Test {
    struct Example has copy, drop { a: address}

    use Std::Debug;
    friend TestAddr::AnotherTest;

    public fun print() {
        let example = Example { a: @TestAddr};
        Debug::print(&example)
    }
}
```

Because named addresses only exist at the source language level and during compilation,
named addresses will be fully substituted for their value at the bytecode
level. For example if we had the following code:

```move=
script {
    fun example() {
        MyAddr::M::foo(@MyAddr);
    }
}
```

and we compiled it with `MyAddr` set to `0xC0FFEE`, then it would be equivalent
to the following operationally:

```move=
script {
    fun example() {
        0xC0FFEE::M::foo(@0xC0FFEE);
    }
}
```

However at the source level, these _are not equivalent_—the function
`M::foo` _must_ be accessed through the `MyAddr` named address, and not through
the numerical value assigned to that address.

Module names can start with letters `a` to `z` or letters `A` to `Z`. After the first character, module names can contain underscores `_`, letters `a` to `z`, letters `A` to `Z`, or digits `0` to `9`.

```move
module my_module {}
module FooBar42 {}
```

Typically, module names start with an uppercase letter. A module named `MyModule` should be stored in a source file named `MyModule.move`.

All elements inside a `module` block can appear in any order.
Fundamentally, a module is a collection of [`types`](./structs-and-resources.md) and [`functions`](./functions.md).
[Uses](./uses.md) import types from other modules.
[Friends](./friends.md) specify a list of trusted modules.
[Constants](./constants.md) define private constants that can be used in the functions of a module.
