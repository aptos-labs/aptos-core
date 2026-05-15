# Primitive Types

## Integers

Move supports six unsigned integer types: `u8`, `u16`, `u32`, `u64`, `u128`, and `u256`. Values of these types range from 0 to a maximum that depends on the size of the type.

| Type                             | Value Range              |
| -------------------------------- | ------------------------ |
| Unsigned 8-bit integer, `u8`     | 0 to 2<sup>8</sup> - 1   |
| Unsigned 16-bit integer, `u16`   | 0 to 2<sup>16</sup> - 1  |
| Unsigned 32-bit integer, `u32`   | 0 to 2<sup>32</sup> - 1  |
| Unsigned 64-bit integer, `u64`   | 0 to 2<sup>64</sup> - 1  |
| Unsigned 128-bit integer, `u128` | 0 to 2<sup>128</sup> - 1 |
| Unsigned 256-bit integer, `u256` | 0 to 2<sup>256</sup> - 1 |

_Since version 2.3_, Move also supports signed integer types:

| Type                           | Value Range                             |
| ------------------------------ | --------------------------------------- |
| Signed 8-bit integer, `i8`     | -2<sup>7</sup> to 2<sup>7</sup> - 1     |
| Signed 16-bit integer, `i16`   | -2<sup>15</sup> to 2<sup>15</sup> - 1   |
| Signed 32-bit integer, `i32`   | -2<sup>31</sup> to 2<sup>31</sup> - 1   |
| Signed 64-bit integer, `i64`   | -2<sup>63</sup> to 2<sup>63</sup> - 1   |
| Signed 128-bit integer, `i128` | -2<sup>127</sup> to 2<sup>127</sup> - 1 |
| Signed 256-bit integer, `i256` | -2<sup>255</sup> to 2<sup>255</sup> - 1 |

### Literals

Literal values for these types are specified either as a sequence of digits (e.g., `112`) or as hex literals, e.g., `0xFF`. The type of the literal can optionally be added as a suffix, e.g., `112u8`. If the type is not specified, the compiler will try to infer the type from the context where the literal is used. If the type cannot be inferred, it is assumed to be `u64`.

Number literals can be separated by underscores for grouping and readability (e.g., `1_234_5678`, `1_000u128`, `0xAB_CD_12_35`).

To denote a negative number, prefix it with a `-` sign (e.g., `-112`).

If a literal is too large for its specified (or inferred) size range, an error is reported.

#### Examples

```move
script {
  fun example() {
    // literals with explicit annotations;
    let explicit_u8 = 1u8;
    let explicit_u16 = 1u16;
    let explicit_u32 = 1u32;
    let explicit_u64 = 2u64;
    let explicit_u128 = 3u128;
    let explicit_u256 = 1u256;
    let explicit_u64_underscored = 154_322_973u64;
    let explicit_i8 = -1i8;
    let explicit_i64 = -2i64;

    // literals with simple inference
    let simple_u8: u8 = 1;
    let simple_u16: u16 = 1;
    let simple_u32: u32 = 1;
    let simple_u64: u64 = 2;
    let simple_u128: u128 = 3;
    let simple_u256: u256 = 1;

    // literals with more complex inference
    let complex_u8 = 1; // inferred: u8
    // right hand argument to shift must be u8
    let _unused = 10 << complex_u8;

    let x: u8 = 38;
    let complex_u8 = 2; // inferred: u8
    // arguments to `+` must have the same type
    let _unused = x + complex_u8;

    let complex_u128 = 133_876; // inferred: u128
    // inferred from function argument type
    function_that_takes_u128(complex_u128);

    // literals can be written in hex
    let hex_u8: u8 = 0x1;
    let hex_u16: u16 = 0x1BAE;
    let hex_u32: u32 = 0xDEAD80;
    let hex_u64: u64 = 0xCAFE;
    let hex_u128: u128 = 0xDEADBEEF;
    let hex_u256: u256 = 0x1123_456A_BCDE_F;
  }
}
```

### Operations

#### Arithmetic

Each of these types supports the same set of checked arithmetic operations. For all of these operations, both arguments (the left and right side operands) _must_ be of the same type. If you need to operate over values of different types, you will need to first perform a [cast](#casting). Similarly, if you expect the result of the operation to be too large for the integer type, perform a [cast](#casting) to a larger size before performing the operation.

All arithmetic operations abort instead of behaving in a way that mathematical integers would not (e.g., overflow, underflow, divide-by-zero).

| Syntax  | Operation           | Aborts If                                      |
| ------- | ------------------- | ---------------------------------------------- |
| `a + b` | addition            | Result is too large/small for the integer type |
| `a - b` | subtraction         | Result is less than zero                       |
| `a * b` | multiplication      | Result is too large/small for the integer type |
| `a % b` | modular division    | The divisor is `0`                             |
| `a / b` | truncating division | The divisor is `0`, or the result overflows    |
| `-a`    | negation            | Negated result too large (e.g. `-MIN_I64`)     |

#### Bitwise

The _unsigned_ integer types support the following bitwise operations that treat each number as a series of individual bits, either 0 or 1, instead of as numerical integer values.

Bitwise operations do not abort.

| Syntax | Operation   | Description                                           |
| ------ | ----------- | ----------------------------------------------------- |
| `&`    | bitwise and | Performs a boolean and for each bit pairwise          |
| `\|`   | bitwise or  | Performs a boolean or for each bit pairwise           |
| `^`    | bitwise xor | Performs a boolean exclusive or for each bit pairwise |

#### Bit Shifts

Similar to the bitwise operations, each _unsigned_ integer type supports bit shifts. But unlike the other operations, the right-hand side operand (how many bits to shift by) must _always_ be a `u8` and need not match the left side operand (the number you are shifting).

Bit shifts abort if the number of bits to shift by is greater than or equal to `8`, `16`, `32`, `64`, `128` or `256` for `u8`, `u16`, `u32`, `u64`, `u128` and `u256` respectively.

| Syntax | Operation   | Aborts if                                                                           |
| ------ | ----------- | ----------------------------------------------------------------------------------- |
| `<<`   | shift left  | Number of bits to shift by is greater than or equal to the size of the integer type |
| `>>`   | shift right | Number of bits to shift by is greater than or equal to the size of the integer type |

#### Comparisons

All integer types support the ["comparison"](./equality-and-comparison.md) operations. Both arguments need to be of the same type. If you need to compare integers of different types, you will need to [cast](#casting) one of them first.

Comparison operations do not abort.

| Syntax | Operation                |
| ------ | ------------------------ |
| `<`    | less than                |
| `>`    | greater than             |
| `<=`   | less than or equal to    |
| `>=`   | greater than or equal to |

#### Equality

Like all types with [`drop`](./generics-and-abilities.md) in Move, all integer types support the ["equal"](./equality-and-comparison.md) and ["not equal"](./equality-and-comparison.md) operations. Both arguments need to be of the same type. If you need to compare integers of different types, you will need to [cast](#casting) one of them first.

Equality operations do not abort.

| Syntax | Operation |
| ------ | --------- |
| `==`   | equal     |
| `!=`   | not equal |

For more details see the section on [equality](./equality-and-comparison.md)

### Casting

Integer types of one size can be cast to integer types of another size. Integers are the only types in Move that support casting.

Casts _do not_ truncate. Casting will abort if the result is too large or too small for the specified type.

| Syntax     | Operation                                            | Aborts if                                           |
| ---------- | ---------------------------------------------------- | --------------------------------------------------- |
| `(e as T)` | Cast integer expression `e` into an integer type `T` | `e` is too large or too small to represent as a `T` |

Any integer can be cast into any other integer type, including signed to unsigned and unsigned to signed, provided the target type is able to represent the source value.

For example:

- `(x as u8)`
- `(y as u16)`
- `(873u16 as u32)`
- `(2u8 as u64)`
- `(1 + 3 as u128)`
- `(4/2 + 12345 as u256)`

Notice that since language version 2.0, casts don't always need to be in parentheses. Thus, `x as u8` is a valid expression.

### Ownership

As with the other scalar values built-in to the language, integer values are implicitly copyable, meaning they can be copied without an explicit instruction such as [`copy`](./variables.md#move-and-copy).

## Bool

`bool` is Move's primitive type for boolean `true` and `false` values.

### Literals

Literals for `bool` are either `true` or `false`.

### Operations

#### Logical

`bool` supports three logical operations:

| Syntax | Description                  | Equivalent Expression                            |
| ------ | ---------------------------- | ------------------------------------------------ |
| `&&`   | short-circuiting logical and | `p && q` is equivalent to `if (p) q else false`  |
| `\|\|` | short-circuiting logical or  | `p \|\| q` is equivalent to `if (p) true else q` |
| `!`    | logical negation             | `!p` is equivalent to `if (p) false else true`   |

#### Control Flow

`bool` values are used in several of Move's control-flow constructs:

- [`if (bool) { ... }`](./conditionals-and-loops.md)
- [`while (bool) { .. }`](./conditionals-and-loops.md)
- [`assert!(bool, u64)`](./abort-and-assert.md)

### Ownership

As with the other scalar values built into the language, boolean values are implicitly copyable,
meaning they can be copied without an explicit instruction such as
[`copy`](./variables.md#move-and-copy).

## Address

`address` is a built-in type in Move that is used to represent locations (sometimes called accounts) in global storage. An `address` value is a 256-bit (32-byte) identifier. At a given address, two things can be stored: [Modules](./modules-and-packages.md) and [Resources](./structs-and-enums.md).

Although an `address` is a 256-bit integer under the hood, Move addresses are intentionally opaque---they cannot be created from integers, they do not support arithmetic operations, and they cannot be modified. Even though there might be interesting programs that would use such a feature (e.g., pointer arithmetic in C fills a similar niche), Move does not allow this dynamic behavior because it has been designed from the ground up to support static verification.

You can use runtime address values (values of type `address`) to access resources at that address. You _cannot_ access modules at runtime via address values.

### Addresses and Their Syntax

Addresses come in two flavors, named or numerical. The syntax for a named address follows the
same rules for any named identifier in Move. The syntax of a numerical address is not restricted
to hex-encoded values, and any valid [`u256` numerical value](./primitive-types.md) can be used as an
address value, e.g., `42`, `0xCAFE`, and `2021` are all valid numerical address
literals.

To distinguish when an address is being used in an expression context or not, the
syntax when using an address differs depending on the context where it's used:

- When an address is used as an expression the address must be prefixed by the `@` character, i.e., [`@<numerical_value>`](./primitive-types.md) or `@<named_address_identifier>`.
- Outside of expression contexts, the address may be written without the leading `@` character, i.e., [`<numerical_value>`](./primitive-types.md) or `<named_address_identifier>`.

In general, you can think of `@` as an operator that takes an address from being a namespace item to being an expression item.

### Named Addresses

Named addresses are a feature that allows identifiers to be used in place of
numerical values in any spot where addresses are used, and not just at the
value level. Named addresses are declared and bound as top-level elements
(outside of modules and scripts) in Move Packages, or passed as arguments
to the Move compiler.

Named addresses only exist at the source language level and will be fully
substituted for their value at the bytecode level. Because of this, modules
and module members _must_ be accessed through the module's named address
and not through the numerical value assigned to the named address during
compilation, e.g., `use my_addr::foo` is _not_ equivalent to `use 0x2::foo`
even if the Move program is compiled with `my_addr` set to `0x2`. This
distinction is discussed in more detail in the section on [Modules and Scripts](./modules-and-packages.md).

#### Examples

```move
script {
  fun example() {
    let a1: address = @0x1; // shorthand for 0x0000000000000000000000000000000000000000000000000000000000000001
    let a2: address = @0x42; // shorthand for 0x0000000000000000000000000000000000000000000000000000000000000042
    let a3: address = @0xDEADBEEF; // shorthand for 0x00000000000000000000000000000000000000000000000000000000DEADBEEF
    let a4: address = @0x000000000000000000000000000000000000000000000000000000000000000A;
    let a5: address = @std; // Assigns `a5` the value of the named address `std`
    let a6: address = @66;
    let a7: address = @0x42;
  }
}

module 66::some_module {   // Not in expression context, so no @ needed
    use 0x1::other_module; // Not in expression context so no @ needed
    use std::vector;       // Can use a named address as a namespace item when using other modules
    ...
}

module std::other_module {  // Can use a named address as a namespace item to declare a module
    ...
}
```

### Global Storage Operations

The primary purpose of `address` values is to interact with the global storage operations.

`address` values are used with the `exists`, `borrow_global`, `borrow_global_mut`, and `move_from` [operations](./global-storage.md).

The only global storage operation that _does not_ use `address` is `move_to`, which uses [`signer`](./primitive-types.md).

### Ownership

As with the other scalar values built-in to the language, `address` values are implicitly copyable, meaning they can be copied without an explicit instruction such as [`copy`](./variables.md#move-and-copy).

## Signer

`signer` is a built-in Move resource type. A `signer` is a
[capability](https://en.wikipedia.org/wiki/Object-capability_model) that allows the holder to act on
behalf of a particular `address`. You can think of the native implementation as being:

```move
module 0x1::signer {
  struct signer has drop { a: address }
}
```

A `signer` is somewhat similar to a Unix [UID](https://en.wikipedia.org/wiki/User_identifier) in
that it represents a user authenticated by code _outside_ of Move (e.g., by checking a cryptographic
signature or password).

### Comparison to `address`

A Move program can create any `address` value without special permission using address literals:

```move
script {
  fun example() {
    let a1 = @0x1;
    let a2 = @0x2;
    // ... and so on for every other possible address
  }
}
```

However, `signer` values are special because they cannot be created via literals or
instructions — only by the Move VM. Before the VM runs a script with parameters of type `signer`, it
will automatically create `signer` values and pass them into the script:

```move
script {
    use std::signer;
    fun main(s: signer) {
        assert!(signer::address_of(&s) == @0x42, 0);
    }
}
```

This script will abort with code `0` if the script is sent from any address other than `0x42`.

A Move script can have an arbitrary number of `signer`s as long as the `signer`s are a prefix
to any other arguments. In other words, all of the `signer` arguments must come first:

```move
script {
    use std::signer;
    fun main(s1: signer, s2: signer, x: u64, y: u8) {
        // ...
    }
}
```

This is useful for implementing _multi-signer scripts_ that atomically act with the authority of
multiple parties. For example, an extension of the script above could perform an atomic currency
swap between `s1` and `s2`.

### `signer` Operators

The `std::signer` standard library module provides two utility functions over `signer` values:

| Function                                    | Description                                                    |
| ------------------------------------------- | -------------------------------------------------------------- |
| `signer::address_of(&signer): address`      | Return the `address` wrapped by this `&signer`.                |
| `signer::borrow_address(&signer): &address` | Return a reference to the `address` wrapped by this `&signer`. |

In addition, the `move_to<T>(&signer, T)` [global storage operator](./global-storage.md)
requires a `&signer` argument to publish a resource `T` under `signer.address`'s account. This
ensures that only an authenticated user can elect to publish a resource under their `address`.

### Ownership

Unlike simple scalar values, `signer` values are not copyable, meaning they cannot be copied from
any operation whether it be through an explicit [`copy`](./variables.md#move-and-copy) instruction
or through a [dereference `*`](./references.md#reading-and-writing-through-references).
