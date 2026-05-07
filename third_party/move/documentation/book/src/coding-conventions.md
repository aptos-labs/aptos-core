# Move Coding Conventions

This section lays out some basic coding conventions for Move that the Move team has found helpful. These are only recommendations, and you should feel free to use other formatting guidelines and conventions if you have a preference for them.

## Naming

- **Module names**: should be lowercase snake case, e.g., `fixed_point32`, `vector`.
- **Type names**: should be camel case if they are not a native type, e.g., `Coin`, `RoleId`.
- **Function names**: should be lowercase snake case, e.g., `destroy_empty`.
- **Constant names**: should be upper camel case and begin with an `E` if they represent error codes (e.g., `EIndexOutOfBounds`) and upper snake case if they represent a non-error value (e.g., `MIN_STAKE`).
- **Generic type names**: should be descriptive, or anti-descriptive where appropriate, e.g., `T` or `Element` for the Vector generic type parameter. Most of the time the "main" type in a module should be the same name as the module e.g., `option::Option`, `fixed_point32::FixedPoint32`.
- **Module file names**: should be the same as the module name e.g., `option.move`.
- **Script file names**: should be lowercase snake case and should match the name of the "main" function in the script.
- **Mixed file names**: If the file contains multiple modules and/or scripts, the file name should be lowercase snake case, where the name does not match any particular module/script inside.

## Imports

- All module `use` statements should be at the top of the module.
- Functions should be imported and used fully qualified from the module in which they are declared, and not imported at the top level.
- Types should be imported at the top-level. Where there are name clashes, `as` should be used to rename the type locally as appropriate.

For example, if there is a module:

```move
module 0x1::foo {
  struct Foo { }
  const CONST_FOO: u64 = 0;
  public fun do_foo(): Foo { Foo{} }
  // ...
}
```

this would be imported and used as:

```move
module 0x1::bar {
  use 0x1::foo::{Self, Foo};

  public fun do_bar(x: u64): Foo {
    if (x == 10) {
      foo::do_foo()
    } else {
      abort 0
    }
  }
  // ...
}
```

And, if there is a local name-clash when importing two modules:

```move
module 0x1::other_foo {
  struct Foo {}
  // ...
}

module 0x1::importer {
  use 0x1::other_foo::Foo as OtherFoo;
  use 0x1::foo::Foo;
  // ...
}
```

## Comments

- Each module, struct, and public function declaration should be commented.
- Move has doc comments `///`, regular single-line comments `//`, block comments `/* */`, and block doc comments `/** */`.
- Starting Aptos CLI 7.4.0, UTF-8 characters are allowed in comments.

### Comments Example

Doc comments must be directly above the item they are commenting on. For example, the following is valid:

```move
/// My awesome module, doc comment can be used here
module 0x42::example { // double slash can be anywhere

  // Double slash can be anywhere

  /// My awesome constant
  const MY_VALUE: u64 = 5;

  /// My awesome error message
  const E_MY_ERROR: u64 = 10;

  #[view]
  /// My awesome view function
  fun show_me_the_money() {
    // ...
  }

  /* Similarly block comments can be anywhere */
}
```

Below are examples of doc comments `///` that will fail:

```move
module 0x42::example {

  /// My awesome view function <- must be below the annotation, right above the thing commented
  #[view]
  fun show_me_the_money() {
    // ...
    /// Within a function
  }

  /// Not attached to anything
}
```

## Formatting

The Move team plans to write an auto-formatter to enforce formatting conventions. However, in the meantime:

- Four space indentation should be used except for `script` and `address` blocks whose contents should not be indented.
- Lines should be broken if they are longer than 100 characters.
- Structs and constants should be declared before all functions in a module.
