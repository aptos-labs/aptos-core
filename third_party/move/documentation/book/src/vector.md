# Vector

`vector<T>` is the only primitive collection type provided by Move. A `vector<T>` is a homogeneous
collection of `T`'s that can grow or shrink by pushing/popping values off the "end".

A `vector<T>` can be instantiated with any type `T`. For example, `vector<u64>`, `vector<address>`,
`vector<0x42::MyModule::MyResource>`, and `vector<vector<u8>>` are all valid vector types.

## Literals

### General `vector` Literals

Vectors of any type can be created with `vector` literals.

| Syntax                | Type                                                                          | Description                                |
| --------------------- | ----------------------------------------------------------------------------- | ------------------------------------------ |
| `vector[]`            | `vector[]: vector<T>` where `T` is any single, non-reference type             | An empty vector                            |
| `vector[e1, ..., en]` | `vector[e1, ..., en]: vector<T>` where `e_i: T` s.t. `0 < i <= n` and `n > 0` | A vector with `n` elements (of length `n`) |

In these cases, the type of the `vector` is inferred, either from the element type or from the
vector's usage. If the type cannot be inferred, or simply for added clarity, the type can be
specified explicitly:

```move
vector<T>[]: vector<T>
vector<T>[e1, ..., en]: vector<T>
```

#### Example Vector Literals

```move
script {
  fun example() {
    (vector[]: vector<bool>);
    (vector[0u8, 1u8, 2u8]: vector<u8>);
    (vector<u128>[]: vector<u128>);
    (vector<address>[@0x42, @0x100]: vector<address>);
  }
}
```

### `vector<u8>` literals

A common use-case for vectors in Move is to represent "byte arrays", which are represented with
`vector<u8>`. These values are often used for cryptographic purposes, such as a public key or a hash
result. These values are so common that specific syntax is provided to make the values more
readable, as opposed to having to use `vector[]` where each individual `u8` value is specified in
numeric form.

There are currently two supported types of `vector<u8>` literals, _byte strings_ and _hex strings_.

#### Byte Strings

Byte strings are quoted string literals prefixed by a `b`, e.g. `b"Hello!\n"`.

These are ASCII encoded strings that allow for escape sequences. Currently, the supported escape
sequences are:

| Escape Sequence | Description                                    |
| --------------- | ---------------------------------------------- |
| `\n`            | New line (or Line feed)                        |
| `\r`            | Carriage return                                |
| `\t`            | Tab                                            |
| `\\`            | Backslash                                      |
| `\0`            | Null                                           |
| `\"`            | Quote                                          |
| `\xHH`          | Hex escape, inserts the hex byte sequence `HH` |

#### Hex Strings

Hex strings are quoted string literals prefixed by a `x`, e.g. `x"48656C6C6F210A"`.

Each byte pair, ranging from `00` to `FF`, is interpreted as a hex-encoded `u8` value. So each byte
pair corresponds to a single entry in the resulting `vector<u8>`.

#### Example String Literals

```move
script {
  fun byte_and_hex_strings() {
    assert!(b"" == x"", 0);
    assert!(b"Hello!\n" == x"48656C6C6F210A", 1);
    assert!(b"\x48\x65\x6C\x6C\x6F\x21\x0A" == x"48656C6C6F210A", 2);
    assert!(
        b"\"Hello\tworld!\"\n \r \\Null=\0" ==
            x"2248656C6C6F09776F726C6421220A200D205C4E756C6C3D00",
        3
    );
  }
}
```

## Operations

`vector` provides several operations via the `std::vector` module in the Move standard
library, as shown below. More operations may be added over time.
Up-to-date documentation on `vector` is in the
[Aptos Framework Book](https://aptos-labs.github.io/framework-book/move-stdlib/vector.html).

| Function                                                                              | Description                                                                                                                                                        | Aborts?                                                  |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------- |
| `vector::empty<T>(): vector<T>`                                                       | Create an empty vector that can store values of type `T`                                                                                                           | Never                                                    |
| `vector::is_empty<T>(self: &vector<T>): bool`                                         | Return `true` if the vector `self` has no elements and `false` otherwise.                                                                                          | Never                                                    |
| `vector::singleton<T>(t: T): vector<T>`                                               | Create a vector of size 1 containing `t`                                                                                                                           | Never                                                    |
| `vector::length<T>(self: &vector<T>): u64`                                            | Return the length of the vector `self`                                                                                                                             | Never                                                    |
| `vector::push_back<T>(self: &mut vector<T>, t: T)`                                    | Add `t` to the end of `self`                                                                                                                                       | Never                                                    |
| `vector::pop_back<T>(self: &mut vector<T>): T`                                        | Remove and return the last element in `self`                                                                                                                       | If `self` is empty                                       |
| `vector::borrow<T>(self: &vector<T>, i: u64): &T`                                     | Return an immutable reference to the element at index `i`                                                                                                          | If `i` is not in bounds                                  |
| `vector::borrow_mut<T>(self: &mut vector<T>, i: u64): &mut T`                         | Return a mutable reference to the element at index `i`                                                                                                             | If `i` is not in bounds                                  |
| `vector::destroy_empty<T>(self: vector<T>)`                                           | Delete `self`                                                                                                                                                      | If `self` is not empty                                   |
| `vector::append<T>(self: &mut vector<T>, other: vector<T>)`                           | Add the elements in `other` to the end of `self`                                                                                                                   | Never                                                    |
| `vector::reverse_append<T>(self: &mut vector<T>, other: vector<T>)`                   | Pushes all of the elements of the `other` vector into the `self` vector, in the reverse order as they occurred in `other`                                          | Never                                                    |
| `vector::contains<T>(self: &vector<T>, e: &T): bool`                                  | Return true if `e` is in the vector `self`. Otherwise, returns `false`                                                                                             | Never                                                    |
| `vector::swap<T>(self: &mut vector<T>, i: u64, j: u64)`                               | Swaps the elements at the `i`th and `j`th indices in the vector `self`                                                                                             | If `i` or `j` is out of bounds                           |
| `vector::reverse<T>(self: &mut vector<T>)`                                            | Reverses the order of the elements in the vector `self` in place                                                                                                   | Never                                                    |
| `vector::reverse_slice<T>(self: &mut vector<T>, l: u64, r: u64)`                      | Reverses the order of the elements `[l, r)` in the vector `self` in place                                                                                          | If `l > r` or if `l` or `r` is out of bounds             |
| `vector::index_of<T>(self: &vector<T>, e: &T): (bool, u64)`                           | Return `(true, i)` if `e` is in the vector `self` at index `i`. Otherwise, returns `(false, 0)`                                                                    | Never                                                    |
| `vector::insert<T>(self: &mut vector<T>, i: u64, e: T)`                               | Insert a new element `e` at position `0 <= i <= length`, using `O(length - i)` time                                                                                | If `i` is out of bounds                                  |
| `vector::remove<T>(self: &mut vector<T>, i: u64): T`                                  | Remove the `i`th element of the vector `self`, shifting all subsequent elements. This is O(n) and preserves ordering of elements in the vector                     | If `i` is out of bounds                                  |
| `vector::swap_remove<T>(self: &mut vector<T>, i: u64): T`                             | Swap the `i`th element of the vector `self` with the last element and then pop the element, This is O(1), but does not preserve ordering of elements in the vector | If `i` is out of bounds                                  |
| `vector::trim<T>(self: &mut vector<T>, new_len: u64): vector<T>`                      | Trim the vector `self` to the smaller size `new_len` and return the evicted elements in order                                                                      | If `new_len > self.length()`                             |
| `vector::trim_reverse<T>(self: &mut vector<T>, new_len: u64): vector<T>`              | Trim the vector `self` to the smaller size `new_len` and return the evicted elements in the reverse order                                                          | If `new_len > self.length()`                             |
| `vector::rotate<T>(self: &mut vector<T>, rot: u64): u64`                              | `rotate(&mut [1, 2, 3, 4, 5], 2) -> [3, 4, 5, 1, 2]` in place, returns the split point i.e., `3` in this example                                                   | If `rot <= self.length()` does not hold                  |
| `vector::rotate_slice<T>(self: &mut vector<T>, left: u64, rot: u64, right: u64): u64` | rotate a slice `[left, right)` with `left <= rot <= right` in place, returns the split point                                                                       | If `left <= rot <= right <= self.length()` does not hold |

Example:

```move
script {
  use std::vector;

  fun example() {
    let v = vector::empty<u64>();
    vector::push_back(&mut v, 5);
    vector::push_back(&mut v, 6);

    assert!(*vector::borrow(&v, 0) == 5, 42);
    assert!(*vector::borrow(&v, 1) == 6, 42);
    assert!(vector::pop_back(&mut v) == 6, 42);
    assert!(vector::pop_back(&mut v) == 5, 42);
  }
}
```

## Index Notation for Vectors

_Since language version 2.0_

Index notation using square brackets (`[]`) is available for vector operations, simplifying syntax
and making programs easier to understand. The index notation is simply syntactic sugar that
is reduced to existing operations by the compiler; the named operations are also still supported.

The table below gives an overview of index notations for vectors:

| Indexing Syntax   | Vector Operation                           |
| ----------------- | ------------------------------------------ |
| `&v[i]`           | `vector::borrow(&v, i)`                    |
| `&mut v[i]`       | `vector::borrow_mut(&mut v, i)`            |
| `v[i]`            | `*vector::borrow(&v, i)`                   |
| `v[i] = x`        | `*vector::borrow_mut(&mut v, i) = x`       |
| `&v[i].field`     | `&vector::borrow(&v, i).field`             |
| `&mut v[i].field` | `&mut vector::borrow_mut(&mut v, i).field` |
| `v[i].field`      | `vector::borrow(&v, i).field`              |
| `v[i].field = x`  | `vector::borrow_mut(&mut v, i).field = x`  |

As an example, here is a bubble sort algorithm for vectors using index notation:

```move
fun bubble_sort(v: vector<u64>) {
  use std::vector;
  let n = vector::length(&v);
  let i = 0;

  while (i < n) {
    let j = 0;
    while (j < n - i - 1) {
      if (v[j] > v[j + 1]) {
        let t = v[j];
        v[j] = v[j + 1];
        v[j + 1] = t;
      };
      j = j + 1;
    };
    i = i + 1;
  };
}
```

## Destroying and copying vectors

Some behaviors of `vector<T>` depend on the abilities of the element type, `T`. For example, vectors
containing elements that do not have `drop` cannot be implicitly discarded like `v` in the example
above — they must be explicitly destroyed with `vector::destroy_empty`.

Note that `vector::destroy_empty` will abort at runtime unless `vec` contains zero elements:

```move
script {
  fun destroy_any_vector<T>(vec: vector<T>) {
    vector::destroy_empty(vec) // deleting this line will cause a compiler error
  }
}
```

But no error would occur for dropping a vector that contains elements with `drop`:

```move
script {
  fun destroy_droppable_vector<T: drop>(vec: vector<T>) {
    // valid!
    // nothing needs to be done explicitly to destroy the vector
  }
}
```

Similarly, vectors cannot be copied unless the element type has `copy`. In other words, a
`vector<T>` has `copy` if and only if `T` has `copy`.

For more details see the sections on [type abilities](./generics-and-abilities.md) and [generics](./generics-and-abilities.md).

## Ownership

As mentioned [above](#destroying-and-copying-vectors), `vector` values can be copied only if the
elements can be copied.
