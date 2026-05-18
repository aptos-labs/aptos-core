# Constants

Constants are a way of giving a name to shared, static values inside of a `module` or `script`.

The constant's value must be known at compile time. The constant's value is stored in the compiled
module or script, and each time the constant is used, a new copy of that value is made.

## Declaration

Constant declarations begin with the `const` keyword, followed by a name, a type, and a value. They
can exist in either a script or module

```text
const <name>: <type> = <expression>;
```

For example

```move
script {
  const MY_ERROR_CODE: u64 = 0;

  fun main(input: u64) {
    assert!(input > 0, MY_ERROR_CODE);
  }
}

module 0x42::example {
  const MY_ADDRESS: address = @0x42;

  public fun permissioned(s: &signer) {
    assert!(std::signer::address_of(s) == MY_ADDRESS, 0);
  }
}
```

## Naming

Constants must start with a capital letter `A` to `Z`. After the first letter, constant names can
contain underscores `_`, letters `a` to `z`, letters `A` to `Z`, or digits `0` to `9`.

```move
script {
  const FLAG: bool = false;
  const MY_ERROR_CODE: u64 = 0;
  const ADDRESS_42: address = @0x42;
}
```

Even though you can use letters `a` to `z` in a constant, the
[general style guidelines](./coding-conventions.md) are to use just uppercase letters `A` to `Z`,
with underscores `_` between each word.

This naming restriction of starting with `A` to `Z` is in place to give room for future language
features. It may or may not be removed later.

## Visibility

_Since language version 2.4_

By default, a constant is module-private: it can be read only inside the module that declares it.
Move 2.4 introduces explicit visibility modifiers that allow constants to be read from other
modules.

### Visibility Levels

Three modifiers are available, mirroring the function visibility keywords:

| Modifier  | Accessible from                                     |
| --------- | --------------------------------------------------- |
| `public`  | Any module                                          |
| `package` | All modules in the same package (same address)      |
| `friend`  | Modules declared as `friend` in the defining module |

`public(package)` and `public(friend)` are accepted as aliases for `package` and `friend`
respectively, but are discouraged and will be deprecated. Prefer the shorthand forms.

```move
module 0x42::m {
    friend 0x42::n;

    public const PUB: u64 = 10;
    package const PKG: u64 = 20;
    friend const FRD: u64 = 30;
    const PRIV: u64 = 40; // module-private (default)
}

module 0x42::n {
    use 0x42::m;

    public fun read(): u64 {
        m::PUB + m::PKG + m::FRD // all valid: same package, and `n` is a friend of `m`
    }
}

module 0x43::other {
    use 0x42::m;

    public fun read(): u64 {
        m::PUB         // valid: `public` is accessible from any module
        // m::PKG      // ERROR: different package
        // m::FRD      // ERROR: `other` is not a friend of `m`
        // m::PRIV     // ERROR: module-private
    }
}
```

### Performance Consideration

Cross-module constant reads are currently compiled into a synthetic accessor function call rather
than a direct `LdConst` bytecode instruction. Thus, they are not a zero-cost abstraction yet: they
are more expensive than the equivalent read inside the defining module. This is expected to change
in the future with VM improvements. In the meantime, use visibility modifiers only when the
cross-module access is genuinely needed.

### Upgrade Behavior

Non-private constants can have their value changed in a module upgrade. Because cross-module reads
go through the synthetic accessor function, consuming modules automatically observe the new value
after the defining module is upgraded — they do not need to be recompiled or republished.

### Restrictions

- Visibility modifiers are not allowed on constants declared inside a `script { ... }` block.
- A cross-module constant read cannot appear in another constant's initializer. Same-module
  references are fine:

  ```move
  module 0x42::m {
      public const A: u64 = 10;
      const B: u64 = A + 1; // OK: same module
  }

  module 0x42::n {
      use 0x42::m;
      public const C: u64 = m::A + 1; // ERROR: cross-module const read in a const initializer
  }
  ```

- A `friend` or `package` constant cannot be read inside a `public` or `friend` `inline` function.
  Such an inline function can be expanded into a module that lacks the required visibility, which
  would leave an inaccessible accessor call in the caller's bytecode. Private `inline` functions
  and `package inline` functions are unaffected, because their expansions cannot escape the
  visibility scope.

## Valid Expressions

Currently, constants are limited to the primitive types `bool`, `u8`, `u16`, `u32`, `u64`, `u128`, `u256`, `address`, and
`vector<u8>`. Future support for other `vector` values (besides the "string"-style literals) will
come later.

### Values

Commonly, `const`s are assigned a simple value, or literal, of their type. For example

```move
script {
  const MY_BOOL: bool = false;
  const MY_ADDRESS: address = @0x70DD;
  const BYTES: vector<u8> = b"hello world";
  const HEX_BYTES: vector<u8> = x"DEADBEEF";
}
```

### Complex Expressions

In addition to literals, constants can include more complex expressions, as long as the compiler is
able to reduce the expression to a value at compile time.

Currently, equality operations, all boolean operations, all bitwise operations, and all arithmetic
operations can be used.

```move
script {
  const RULE: bool = true && false;
  const CAP: u64 = 10 * 100 + 1;
  const SHIFTY: u8 = {
    (1 << 1) * (1 << 2) * (1 << 3) * (1 << 4)
  };
  const HALF_MAX: u128 = 340282366920938463463374607431768211455 / 2;
  const REM: u256 = 57896044618658097711785492504343953926634992332820282019728792003956564819968 % 654321;
  const EQUAL: bool = 1 == 1;
}
```

If the operation results in a runtime exception, the compiler will give an error that it is
unable to generate the constant's value:

```move
script {
  const DIV_BY_ZERO: u64 = 1 / 0; // error!
  const SHIFT_BY_A_LOT: u64 = 1 << 100; // error!
  const NEGATIVE_U64: u64 = 0 - 1; // error!
}
```

Note that constants cannot currently refer to other constants. This feature, along with support for
other expressions, will be added in the future.

## Builtin Constants

Builtin constants are predefined named values which can be used from anywhere in the code. The following constants are supported:

_since language version 2.2_

| Name                            | Value                                                |
| ------------------------------- | ---------------------------------------------------- |
| `__COMPILE_FOR_TESTING__: bool` | `true` when compiling unit tests, `false`  otherwise |

_since language version 2.3_

| Name             | Value               |
| ---------------- | ------------------- |
| `MAX_U8: u8`     | 2<sup>8</sup> - 1   |
| `MAX_U16: u16`   | 2<sup>16</sup> - 1  |
| `MAX_U32: u32`   | 2<sup>32</sup> - 1  |
| `MAX_U64: u64`   | 2<sup>64</sup> - 1  |
| `MAX_U128: u128` | 2<sup>128</sup> - 1 |
| `MAX_U256: u256` | 2<sup>256</sup> - 1 |
| `MAX_I8: i8`     | 2<sup>7</sup> - 1   |
| `MAX_I16: i16`   | 2<sup>15</sup> - 1  |
| `MAX_I32: i32`   | 2<sup>31</sup> - 1  |
| `MAX_I64: i64`   | 2<sup>63</sup> - 1  |
| `MAX_I128: i128` | 2<sup>127</sup> - 1 |
| `MAX_I256: i256` | 2<sup>255</sup> - 1 |
| `MIN_I8: i8`     | -2<sup>7</sup>      |
| `MIN_I16: i16`   | -2<sup>15</sup>     |
| `MIN_I32: i32`   | -2<sup>31</sup>     |
| `MIN_I64: i64`   | -2<sup>63</sup>     |
| `MIN_I128: i128` | -2<sup>127</sup>    |
| `MIN_I256: i256` | -2<sup>255</sup>    |

A builtin constant can be shadowed by a user declaration. For example, the below code is valid in language version 2.3, and the builtin constant will simply be shadowed:

```move
module 0x44::m {
   const MAX_U8: u8 = 255; // User defined constant shadowing builtin constant
}
```
