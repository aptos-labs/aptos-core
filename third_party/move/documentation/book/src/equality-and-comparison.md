# Equality and Comparison

## Equality

Move supports two equality operations `==` and `!=`

### Operations

| Syntax | Operation | Description                                                                 |
| ------ | --------- | --------------------------------------------------------------------------- |
| `==`   | equal     | Returns `true` if the two operands have the same value, `false` otherwise   |
| `!=`   | not equal | Returns `true` if the two operands have different values, `false` otherwise |

#### Typing

Both the equal (`==`) and not-equal (`!=`) operations only work if both operands are the same type

```move
script {
  fun example() {
    0 == 0; // `true`
    1u128 == 2u128; // `false`
    b"hello" != x"00"; // `true`
  }
}
```

Equality and non-equality also work over user-defined types!

```move
module 0x42::example {
    struct S has copy, drop { f: u64, s: vector<u8> }

    fun always_true(): bool {
        let s = S { f: 0, s: b"" };
        // parens are not needed but added for clarity in this example
        (copy s) == s
    }

    fun always_false(): bool {
        let s = S { f: 0, s: b"" };
        // parens are not needed but added for clarity in this example
        (copy s) != s
    }
}
```

If the operands have different types, there is a type checking error

```move
script {
  fun example() {
    1u8 == 1u128; // ERROR!
    //     ^^^^^ expected an argument of type 'u8'
    b"" != 0; // ERROR!
    //     ^ expected an argument of type 'vector<u8>'
  }
}
```

#### Typing with references

When comparing [references](./references.md), the type of the reference (immutable or mutable) does
not matter. This means that you can compare an immutable `&` reference with a mutable one `&mut` of
the same underlying type.

```move
script {
  fun example() {
    let i = &0;
    let m = &mut 1;

    i == m; // `false`
    m == i; // `false`
    m == m; // `true`
    i == i; // `true`
  }
}
```

The above is equivalent to applying an explicit freeze to each mutable reference where needed

```move
script {
  fun example() {
    let i = &0;
    let m = &mut 1;

    i == freeze(m); // `false`
    freeze(m) == i; // `false`
    m == m; // `true`
    i == i; // `true`
  }
}
```

But again, the underlying type must be the same type

```move
script {
  fun example() {
    let i = &0;
    let s = &b"";

    i == s; // ERROR!
    //   ^ expected an argument of type '&u64'
  }
}
```

### Restrictions

Both `==` and `!=` consume the value when comparing them. As a result, the type system enforces that
the type must have [`drop`](./generics-and-abilities.md). Recall that without the
[`drop` ability](./generics-and-abilities.md), ownership must be transferred by the end of the function, and such
values can only be explicitly destroyed within their declaring module. If these were used directly
with either equality `==` or non-equality `!=`, the value would be destroyed which would break
[`drop` ability](./generics-and-abilities.md) safety guarantees!

```move
module 0x42::example {
  struct Coin has store { value: u64 }
  fun invalid(c1: Coin, c2: Coin) {
    c1 == c2 // ERROR!
//  ^^    ^^ These resources would be destroyed!
  }
}
```

But, a programmer can _always_ borrow the value first instead of directly comparing the value, and
reference types have the [`drop` ability](./generics-and-abilities.md). For example

```move
module 0x42::example {
  struct Coin has store { value: u64 }
  fun swap_if_equal(c1: Coin, c2: Coin): (Coin, Coin) {
    let are_equal = &c1 == &c2; // valid
    if (are_equal) (c2, c1) else (c1, c2)
  }
}
```

### Avoid Extra Copies

While a programmer _can_ compare any value whose type has [`drop`](./generics-and-abilities.md), a programmer
should often compare by reference to avoid expensive copies.

```move
script {
  fun example() {
    let v1: vector<u8> = function_that_returns_vector();
    let v2: vector<u8> = function_that_returns_vector();
    assert!(copy v1 == copy v2, 42);
    //     ^^^^       ^^^^
    use_two_vectors(v1, v2);

    let s1: Foo = function_that_returns_large_struct();
    let s2: Foo = function_that_returns_large_struct();
    assert!(copy s1 == copy s2, 42);
    //     ^^^^       ^^^^
    use_two_foos(s1, s2);
  }
}
```

This code is perfectly acceptable (assuming `Foo` has [`drop`](./generics-and-abilities.md)), just not efficient.
The highlighted copies can be removed and replaced with borrows

```move
script {
  fun example() {
    let v1: vector<u8> = function_that_returns_vector();
    let v2: vector<u8> = function_that_returns_vector();
    assert!(&v1 == &v2, 42);
    //     ^      ^
    use_two_vectors(v1, v2);

    let s1: Foo = function_that_returns_large_struct();
    let s2: Foo = function_that_returns_large_struct();
    assert!(&s1 == &s2, 42);
    //     ^      ^
    use_two_foos(s1, s2);
  }
}
```

The efficiency of the `==` itself remains the same, but the `copy`s are removed and thus the program
is more efficient.

## Comparison

Move supports four comparison operations `<`, `>`, `<=`, and `>=`.

### Operations

| Syntax | Operation                |
| ------ | ------------------------ |
| `<`    | less than                |
| `>`    | greater than             |
| `<=`   | less than or equal to    |
| `>=`   | greater than or equal to |

#### Typing

Comparison operations only work if both operands have the same type.

```move
script {
  fun example() {
    0 >= 0; // `true`
    1u128 > 2u128; // `false`
  }
}
```

If the operands have different types, there is a type checking error.

```move
script {
  fun example() {
    1u8 >= 1u128; // ERROR!
    //     ^^^^^ expected an argument of type `u8`
  }
}
```

Prior to language version 2.2, comparison operations only worked with integer types. _Since language
version 2.2_, comparison operations work with all types.

| Type           | Semantics                                                                                                   |
| -------------- | ----------------------------------------------------------------------------------------------------------- |
| integer        | compare by the numerical value                                                                              |
| `bool`         | `true` being larger than `false`                                                                            |
| `address`      | compare as 256-bit unsigned integers                                                                        |
| `signer`       | compare by the `address` wrapped by the `signer`                                                            |
| `struct`       | compare by field values first, and then by the number of fields.                                            |
| `vector`       | compare by element values first, and then by the number of elements                                         |
| function value | compare in order by module address, module name, function name, argument type list, and captured value list |
| reference      | compare by the value being referenced                                                                       |

```move
module 0x42::example {
    struct S has copy, drop { f: u64, s: vector<u8> }

    fun true_example(): bool {
        let s1 = S { f: 0, s: b"" };
        let s2 = S { f: 1, s: b"" };
        // return true
        s1 < s2
    }

    fun false_example(): bool {
        let s1 = S { f: 0, s: b"abc" };
        let s2 = S { f: 0, s: b"" };
        // return false
        s1 < s2
    }
}
```

#### Typing with references

When comparing [references](./references.md), the values being referenced are compared. The type of the reference (immutable or mutable) does
not matter. This means that you can compare an immutable `&` reference with a mutable one `&mut` of
the same underlying type.

```move
script {
  fun example() {
    let i = &0u64;
    let m = &mut 1u64;

    i > m; // `false`
    m < i; // `false`
    m >= m; // `true`
    i <= i; // `true`
  }
}
```

The above is equivalent to applying an explicit freeze to each mutable reference where needed:

```move
script {
  fun example() {
    let i = &0u64;
    let m = &mut 1u64;

    i > freeze(m); // `false`
    freeze(m) < i; // `false`
    m >= m; // `true`
    i <= i; // `true`
  }
}
```

But again, the underlying type must be the same.

```move
script {
  fun example() {
    let i = &0u64;
    let s = &b"";

    i > s; // ERROR!
    //   ^ expected an argument of type '&u64'
  }
}
```

### Comparing to `==` and `!=`

Comparison operations consume operands for integers but automatically borrow them for non-integer types.
This differs from the [equality `==` and inequality `!=`](./equality-and-comparison.md#restrictions) operations, which always consume their operands
and mandate the [`drop` ability](./generics-and-abilities.md).

```move
module 0x42::example {
  struct Coin has store { value: u64 }
  fun invalid(c1: Coin, c2: Coin) {
    c1 <= c2 // OK!
    c1 == c2 // ERROR!
//  ^^    ^^ These resources would be destroyed!
  }
}
```
