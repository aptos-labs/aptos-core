# Abort and Assert

[`return`](./functions.md) and `abort` are two control flow constructs that end execution, one for
the current function and one for the entire transaction.

More information on [`return` can be found in the linked section](./functions.md).

## `abort`

`abort` is an expression that takes one argument, which is either an **abort code** of type `u64` or
an **abort message** of type `vector<u8>` (since Move 2.4). For example:

```move
abort 42
abort b"something went wrong"
```

The `abort` expression halts execution of the current function and reverts all changes made to global
state by the current transaction. There is no mechanism for "catching" or otherwise handling an
`abort`.

Luckily, in Move transactions are all or nothing, meaning any changes to global storage are made all
at once only if the transaction succeeds. Because of this transactional commitment of changes, after
an abort there is no need to worry about backing out changes. While this approach is lacking in
flexibility, it is incredibly simple and predictable.

Similar to [`return`](./functions.md), `abort` is useful for exiting control flow when some
condition cannot be met.

In this example, the function will pop two items off of the vector, but will abort early if the
vector does not have two items

```move
script {
  use std::vector;
  fun pop_twice<T>(v: &mut vector<T>): (T, T) {
      if (vector::length(v) < 2) abort 42;

      (vector::pop_back(v), vector::pop_back(v))
  }
}
```

This is even more useful deep inside a control-flow construct. For example, this function checks
that all numbers in the vector are less than the specified `bound`, and aborts otherwise:

```move
script {
  use std::vector;
  fun check_vec(v: &vector<u64>, bound: u64) {
      let i = 0;
      let n = vector::length(v);
      while (i < n) {
          let cur = *vector::borrow(v, i);
          if (cur > bound) abort 42;
          i = i + 1;
      }
  }
}
```

### Abort messages

_Since language version 2.4_

Instead of a numeric code, `abort` accepts an expression of type `vector<u8>` carrying a
human-readable message. The VM imposes two requirements on the message, checked when the `abort` is
executed; failing either causes the transaction to fail with a VM error instead of the user-supplied
message:

- The message must be valid UTF-8; otherwise the transaction fails with `INVALID_ABORT_MESSAGE`.
- The message must be at most 1024 bytes long; otherwise the transaction fails with
  `ABORT_MESSAGE_LIMIT_EXCEEDED`.

When an `abort` is reached with a message, the VM still reports a module address and a `u64` abort
code, and additionally surfaces the message string in the transaction's error information. The code
in this case is fixed by the compiler to the well-known *unspecified* abort code
`0xCA26CBD9BE0B0000`. In terms of the `std::error` convention, this code has category
`std::error::INTERNAL` and reason `0`; it carries no information of its own and signals that the
attached message is the diagnostic to read.

To build a message from runtime values, format a `String` with the formatting functions in
`std::string_utils` and convert it to bytes with `String::into_bytes`. For example:

```move
abort std::string_utils::format1(&b"insufficient balance: needed {}", amount).into_bytes()
```

In practice, the `assert!` family of macros (described below) handles this boilerplate for you and
is usually the more convenient choice.

### `assert`

`assert` is a builtin, macro-like operation provided by the Move compiler. It checks a boolean
condition and, when the condition is false, aborts the transaction with the supplied diagnostic.
The macro supports four forms, differing in what diagnostic is attached to the abort:

```move
assert!(condition: bool)
assert!(condition: bool, code: u64)
assert!(condition: bool, message: vector<u8>)
assert!(condition: bool, fmt: vector<u8>, arg1: T1, ..., argN: TN) // 1 ≤ N ≤ 4
```

Since the operation is a macro, it must be invoked with the `!`. This is to convey that the
arguments to `assert` are call-by-expression. In other words, `assert` is not a normal function and
does not exist at the bytecode level. For example, the form `assert!(condition, code)`
is replaced inside the compiler with

```move
if (condition) () else abort code
```

`assert` is more commonly used than just `abort` by itself. The `abort` examples above can be
rewritten using `assert`

```move
script {
  use std::vector;
  fun pop_twice<T>(v: &mut vector<T>): (T, T) {
      assert!(vector::length(v) >= 2, 42); // Now uses 'assert'

      (vector::pop_back(v), vector::pop_back(v))
  }
}
```

and

```move
script {
  use std::vector;
  fun check_vec(v: &vector<u64>, bound: u64) {
      let i = 0;
      let n = vector::length(v);
      while (i < n) {
          let cur = *vector::borrow(v, i);
          assert!(cur <= bound, 42); // Now uses 'assert'
          i = i + 1;
      }
  }
}
```

Note that because the operation is replaced with this `if-else`, the argument for the `code` is not
always evaluated. For example:

```move
assert!(true, 1 / 0)
```

Will not result in an arithmetic error, it is equivalent to

```move
if (true) () else (1 / 0)
```

So the arithmetic expression is never evaluated!

#### `assert` without an abort code

_Since language version 2.0_

The abort code may be omitted entirely:

```move
assert!(balance >= amount);
```

In this case the macro aborts with the well-known *unspecified* abort code `0xCA26CBD9BE0B0000`
(see [above](#abort-messages)).

#### `assert` with a message

_Since language version 2.4_

The second argument to `assert!` may also be a `vector<u8>` literal containing an abort message:

```move
assert!(balance >= amount, b"insufficient balance");
```

The compiler distinguishes the code form from the message form by the type of the second argument
(`u64` vs `vector<u8>`). On failure, the macro aborts with the message using the unspecified abort
code `0xCA26CBD9BE0B0000`; the message itself is what conveys diagnostic information.

#### `assert` with a formatted message

_Since language version 2.4_

The macro also accepts a format string followed by 1–4 arguments that are interpolated at runtime:

```move
assert!(idx < len, b"index {} out of bounds for vector of length {}", idx, len);
```

This form expands to:

```move
// assert!(cond, fmt, arg1, ..., argN)
if (cond) () else abort std::string::into_bytes(
    std::string_utils::formatN(&fmt, arg1, ..., argN)
)
```

The format string uses `{}` as the placeholder for each argument; `{{` and `}}` produce literal
braces. Up to four arguments are supported, and the number of placeholders must match the number of
arguments; otherwise the compiler reports an error. Any type with the `drop` ability may be passed
as an argument, and is rendered by `std::string_utils`.

### `assert_eq` and `assert_ne`

_Since language version 2.4_

`assert_eq!` and `assert_ne!` are convenience macros for equality and inequality assertions. They
evaluate each operand exactly once and, on failure, abort with a message that includes both values.
These are modelled on the macros of the same name in Rust.

```move
assert_eq!(left, right)
assert_eq!(left, right, message: vector<u8>)
assert_eq!(left, right, fmt: vector<u8>, arg1: T1, ..., argN: TN) // 1 ≤ N ≤ 4

assert_ne!(left, right)
assert_ne!(left, right, message: vector<u8>)
assert_ne!(left, right, fmt: vector<u8>, arg1: T1, ..., argN: TN) // 1 ≤ N ≤ 4
```

The two-argument form expands roughly to:

```move
let ($left, $right) = (left, right);
if ($left == $right) () else abort std::string::into_bytes(
    std::string_utils::format2(
        &b"assertion `left == right` failed\n  left: {}\n right: {}",
        $left, $right,
    )
)
```

If, for example, `assert_eq!(1, 2)` fails, the transaction aborts with the message:

```text
assertion `left == right` failed
  left: 1
 right: 2
```

When a custom message is supplied, it is rendered (via `string::utf8`) into the assertion message:

```move
assert_eq!(actual, expected, b"custom error message")
```

The formatted-message form interpolates the arguments into the user-supplied format string before
embedding it:

```move
assert_eq!(actual, expected, b"mismatch for key {}", key)
```

`assert_ne!` behaves identically except that the condition is `$left != $right` and the message
reads ``assertion `left != right` failed``.

The two operands are evaluated eagerly and exactly once; any user-supplied format arguments are
only evaluated when the assertion fails. All failures use the unspecified abort code
`0xCA26CBD9BE0B0000`; the diagnostic value is in the attached message.

### Abort codes in the Move VM

When using `abort`, it is important to understand how the `u64` code will be used by the VM.

Normally, after successful execution, the Move VM produces a change-set for the changes made to
global storage (added/removed resources, updates to existing resources, etc.).

If an `abort` is reached, the VM will instead indicate an error. Included in that error will be:

- The module that produced the abort (address and name)
- The abort code, and
- The abort message, if one was provided (Move 2.4+).

For example

```move
module 0x42::example {
  public fun aborts() {
    abort 42
  }
}

script {
  fun always_aborts() {
    0x2::example::aborts()
  }
}
```

If a transaction, such as the script `always_aborts` above, calls `0x2::example::aborts`, the VM
would produce an error that indicated the module `0x2::example` and the code `42`.

For aborts carrying a message, the VM additionally validates that the message is valid UTF-8 and at
most 1024 bytes long. A message that fails either check causes the transaction to fail with a VM
error of `INVALID_ABORT_MESSAGE` or `ABORT_MESSAGE_LIMIT_EXCEEDED`, respectively, instead of the
user-supplied abort message.

This can be useful for having multiple aborts being grouped together inside a module.

In this example, the module has two separate error codes used in multiple functions

```move
module 0x42::example {

  use std::vector;

  const EMPTY_VECTOR: u64 = 0;
  const INDEX_OUT_OF_BOUNDS: u64 = 1;

  // move i to j, move j to k, move k to i
  public fun rotate_three<T>(v: &mut vector<T>, i: u64, j: u64, k: u64) {
    let n = vector::length(v);
    assert!(n > 0, EMPTY_VECTOR);
    assert!(i < n, INDEX_OUT_OF_BOUNDS);
    assert!(j < n, INDEX_OUT_OF_BOUNDS);
    assert!(k < n, INDEX_OUT_OF_BOUNDS);

    vector::swap(v, i, k);
    vector::swap(v, j, k);
  }

  public fun remove_twice<T>(v: &mut vector<T>, i: u64, j: u64): (T, T) {
    let n = vector::length(v);
    assert!(n > 0, EMPTY_VECTOR);
    assert!(i < n, INDEX_OUT_OF_BOUNDS);
    assert!(j < n, INDEX_OUT_OF_BOUNDS);
    assert!(i > j, INDEX_OUT_OF_BOUNDS);

    (vector::remove<T>(v, i), vector::remove<T>(v, j))
  }
}
```

## The type of `abort`

The `abort i` expression can have any type! This is because both constructs break from the normal
control flow, so they never need to evaluate to the value of that type.

The following are not useful, but they will type check

```move
let y: address = abort 0;
```

This behavior can be helpful in situations where you have a branching instruction that produces a
value on some branches, but not all. For example:

```move
script {
  fun example() {
    let b =
        if (x == 0) false
        else if (x == 1) true
        else abort 42;
    //       ^^^^^^^^ `abort 42` has type `bool`
  }
}
```
