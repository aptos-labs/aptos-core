# Generics and Type Abilities

## Generics

Generics can be used to define functions and structs over different input data types. This language feature is sometimes referred to as _parametric polymorphism_. In Move, we will often use the term generics interchangeably with type parameters and type arguments.

Generics are commonly used in library code, such as in vector, to declare code that works over any possible instantiation (that satisfies the specified constraints). In other frameworks, generic code can sometimes be used to interact with global storage in many different ways that all still share the same implementation.

### Declaring Type Parameters

Both functions and structs can take a list of type parameters in their signatures, enclosed by a pair of angle brackets `<...>`.

#### Generic Functions

Type parameters for functions are placed after the function name and before the (value) parameter list. The following code defines a generic identity function that takes a value of any type and returns that value unchanged.

```move
module 0x42::example {
  fun id<T>(x: T): T {
    // this type annotation is unnecessary but valid
    (x: T)
  }
}
```

Once defined, the type parameter `T` can be used in parameter types, return types, and inside the function body.

#### Generic Structs

Type parameters for structs are placed after the struct name, and can be used to name the types of the fields.

```move
module 0x42::example {
  struct Foo<T> has copy, drop { x: T }

  struct Bar<T1, T2> has copy, drop {
    x: T1,
    y: vector<T2>,
  }
}
```

Note that [type parameters do not have to be used](#unused-type-parameters).

### Type Arguments

#### Calling Generic Functions

When calling a generic function, one can specify the type arguments for the function's type parameters in a list enclosed by a pair of angle brackets.

```move
module 0x42::example {
  fun foo() {
    let x = id<bool>(true);
  }
}
```

If you do not specify the type arguments, Move's [type inference](#type-inference) will supply them for you.

#### Using Generic Structs

Similarly, one can attach a list of type arguments for the struct's type parameters when constructing or destructing values of generic types.

```move
module 0x42::example {
  fun foo() {
    let foo = Foo<bool> { x: true };
    let Foo<bool> { x } = foo;
  }
}
```

If you do not specify the type arguments, Move's [type inference](#type-inference) will supply them for you.

#### Type Argument Mismatch

If you specify the type arguments, and they conflict with the actual values supplied, an error will be given:

```move
module 0x42::example {
  fun foo() {
    let x = id<u64>(true); // error! true is not a u64
  }
}
```

and similarly:

```move
module 0x42::example {
  fun foo() {
    let foo = Foo<bool> { x: 0 }; // error! 0 is not a bool
    let Foo<address> { x } = foo; // error! bool is incompatible with address
  }
}
```

### Type Inference

In most cases, the Move compiler will be able to infer the type arguments, so you don't have to write them down explicitly. Here's what the examples above would look like if we omit the type arguments:

```move
module 0x42::example {
  fun foo() {
    let x = id(true);
    //        ^ <bool> is inferred

    let foo = Foo { x: true };
    //           ^ <bool> is inferred

    let Foo { x } = foo;
    //     ^ <bool> is inferred
  }
}
```

Note: when the compiler is unable to infer the types, you'll need to annotate them manually. A common scenario is to call a function with type parameters appearing only at return positions.

```move
module 0x2::m {
  use std::vector;

  fun foo() {
    // let v = vector::new();
    //                    ^ The compiler cannot figure out the element type.

    let v = vector::new<u64>();
    //                 ^~~~~ Must annotate manually.
  }
}
```

However, the compiler will be able to infer the type if that return value is used later in that function:

```move
module 0x2::m {
  use std::vector;

  fun foo() {
    let v = vector::new();
    //                 ^ <u64> is inferred
    vector::push_back(&mut v, 42);
  }
}
```

### Unused Type Parameters

For a struct definition,
an unused type parameter is one that
does not appear in any field defined in the struct,
but is checked statically at compile time.
Move allows unused type parameters so the following struct definition is valid:

```move
module 0x2::m {
  struct Foo<T> {
    foo: u64
  }
}
```

This can be convenient when modeling certain concepts. Here is an example:

```move
module 0x2::m {
  // Currency Specifiers
  struct Currency1 {}
  struct Currency2 {}

  // A generic coin type that can be instantiated using a currency
  // specifier type.
  //   e.g. Coin<Currency1>, Coin<Currency2> etc.
  struct Coin<Currency> has store {
    value: u64
  }

  // Write code generically about all currencies
  public fun mint_generic<Currency>(value: u64): Coin<Currency> {
    Coin { value }
  }

  // Write code concretely about one currency
  public fun mint_concrete(value: u64): Coin<Currency1> {
    Coin { value }
  }
}
```

In this example,
`struct Coin<Currency>` is generic on the `Currency` type parameter,
which specifies the currency of the coin and
allows code to be written either
generically on any currency or
concretely on a specific currency.
This genericity applies even when the `Currency` type parameter
does not appear in any of the fields defined in `Coin`.

#### Phantom Type Parameters

In the example above,
although `struct Coin` asks for the `store` ability,
neither `Coin<Currency1>` nor `Coin<Currency2>` will have the `store` ability.
This is because of the rules for
[Conditional Abilities and Generic Types](./generics-and-abilities.md#conditional-abilities-and-generic-types)
and the fact that `Currency1` and `Currency2` don't have the `store` ability,
despite the fact that they are not even used in the body of `struct Coin`.
This might cause some unpleasant consequences.
For example, we are unable to put `Coin<Currency1>` into a wallet in the global storage.

One possible solution would be to
add spurious ability annotations to `Currency1` and `Currency2`
(i.e., `struct Currency1 has store {}`).
But, this might lead to bugs or security vulnerabilities
because it weakens the types with unnecessary ability declarations.
For example, we would never expect a resource in the global storage to have a field in type `Currency1`,
but this would be possible with the spurious `store` ability.
Moreover, the spurious annotations would be infectious,
requiring many functions generic on the unused type parameter to also include the necessary constraints.

Phantom type parameters solve this problem.
Unused type parameters can be marked as _phantom_ type parameters,
which do not participate in the ability derivation for structs.
In this way,
arguments to phantom type parameters are not considered when deriving the abilities for generic types,
thus avoiding the need for spurious ability annotations.
For this relaxed rule to be sound,
Move's type system guarantees that a parameter declared as `phantom` is either
not used at all in the struct definition, or
it is only used as an argument to type parameters also declared as `phantom`.

##### Declaration

In a struct definition
a type parameter can be declared as phantom by adding the `phantom` keyword before its declaration.
If a type parameter is declared as phantom we say it is a phantom type parameter.
When defining a struct, Move's type checker ensures that every phantom type parameter is either
not used inside the struct definition or
it is only used as an argument to a phantom type parameter.

More formally,
if a type is used as an argument to a phantom type parameter
we say the type appears in _phantom position_.
With this definition in place,
the rule for the correct use of phantom parameters can be specified as follows:
**A phantom type parameter can only appear in phantom position**.

The following two examples show valid uses of phantom parameters.
In the first one,
the parameter `T1` is not used at all inside the struct definition.
In the second one, the parameter `T1` is only used as an argument to a phantom type parameter.

```move
module 0x2::m {
  struct S1<phantom T1, T2> { f: u64 }
  //                ^^
  //                Ok: T1 does not appear inside the struct definition


  struct S2<phantom T1, T2> { f: S1<T1, T2> }
  //                                ^^
  //                                Ok: T1 appears in phantom position
}
```

The following code shows examples of violations of the rule:

```move
module 0x2::m {
  struct S1<phantom T> { f: T }
  //                        ^
  //                        Error: Not a phantom position

  struct S2<T> { f: T }

  struct S3<phantom T> { f: S2<T> }
  //                           ^
  //                           Error: Not a phantom position
}
```

##### Instantiation

When instantiating a struct,
the arguments to phantom parameters are excluded when deriving the struct abilities.
For example, consider the following code:

```move
module 0x2::m {
  struct S<T1, phantom T2> has copy { f: T1 }
  struct NoCopy {}
  struct HasCopy has copy {}
}
```

Consider now the type `S<HasCopy, NoCopy>`.
Since `S` is defined with `copy` and all non-phantom arguments have `copy`
then `S<HasCopy, NoCopy>` also has `copy`.

##### Phantom Type Parameters with Ability Constraints

Ability constraints and phantom type parameters are orthogonal features in the sense that
phantom parameters can be declared with ability constraints.
When instantiating a phantom type parameter with an ability constraint,
the type argument has to satisfy that constraint,
even though the parameter is phantom.
For example, the following definition is perfectly valid:

```move
module 0x2::m {
  struct S<phantom T: copy> {}
}
```

The usual restrictions apply and `T` can only be instantiated with arguments having `copy`.

### Constraints

In the examples above, we have demonstrated how one can use type parameters to define "unknown" types that can be plugged in by callers at a later time. This however means the type system has little information about the type and has to perform checks in a very conservative way. In some sense, the type system must assume the worst case scenario for an unconstrained generic. Simply put, by default generic type parameters have no [abilities](./generics-and-abilities.md).

This is where constraints come into play: they offer a way to specify what properties these unknown types have so the type system can allow operations that would otherwise be unsafe.

#### Declaring Constraints

Constraints can be imposed on type parameters using the following syntax.

```move
// T is the name of the type parameter
T: <ability> (+ <ability>)*
```

The `<ability>` can be any of the four [abilities](./generics-and-abilities.md), and a type parameter can be constrained with multiple abilities at once. So all the following would be valid type parameter declarations:

```move
T: copy
T: copy + drop
T: copy + drop + store + key
```

#### Verifying Constraints

Constraints are checked at call sites so the following code won't compile.

```move
module 0x2::m {
  struct Foo<T: key> { x: T }

  struct Bar { x: Foo<u8> }
  //                  ^ error! u8 does not have 'key'

  struct Baz<T> { x: Foo<T> }
  //                     ^ error! T does not have 'key'
}
```

```move
module 0x2::m {
  struct R {}

  fun unsafe_consume<T>(x: T) {
    // error! x does not have 'drop'
  }

  fun consume<T: drop>(x: T) {
    // valid!
    // x will be dropped automatically
  }

  fun foo() {
    let r = R {};
    consume<R>(r);
    //      ^ error! R does not have 'drop'
  }
}
```

```move
module 0x2::m {
  struct R {}

  fun unsafe_double<T>(x: T) {
    (copy x, x)
    // error! x does not have 'copy'
  }

  fun double<T: copy>(x: T) {
    (copy x, x) // valid!
  }

  fun foo(): (R, R) {
    let r = R {};
    double<R>(r)
    //     ^ error! R does not have 'copy'
  }
}
```

For more information, see the abilities section on [conditional abilities and generic types](./generics-and-abilities.md#conditional-abilities-and-generic-types).

### Limitations on Recursions

#### Recursive Structs

Generic structs cannot contain fields of the same type, either directly or indirectly, even with different type arguments. All the following struct definitions are invalid:

```move
module 0x2::m {
  struct Foo<T> {
    x: Foo<u64> // error! 'Foo' containing 'Foo'
  }

  struct Bar<T> {
    x: Bar<T> // error! 'Bar' containing 'Bar'
  }

  // error! 'A' and 'B' forming a cycle, which is not allowed either.
  struct A<T> {
    x: B<T, u64>
  }

  struct B<T1, T2> {
    x: A<T1>,
    y: A<T2>
  }
}
```

#### Advanced Topic: Type-level Recursions

Move allows generic functions to be called recursively. However, when used in combination with generic structs, this could create an infinite number of types in certain cases, and allowing this means adding unnecessary complexity to the compiler, VM, and other language components. Therefore, such recursions are forbidden.

Allowed:

```move
module 0x2::m {
  struct A<T> {}

  // Finitely many types -- allowed.
  // foo1<T> -> foo1<T> -> foo1<T> -> ... is valid
  fun foo1<T>() {
    foo1<T>();
  }

  // Finitely many types -- allowed.
  // foo2<T> -> foo2<A<u64>> -> foo2<A<u64>> -> ... is valid
  fun foo2<T>() {
    foo2<A<u64>>();
  }
}
```

Not allowed:

```move
module 0x2::m {
  struct A<T> {}

  // Infinitely many types -- NOT allowed.
  // error!
  // foo<T> -> foo<A<T>> -> foo<A<A<T>>> -> ...
  fun foo<T>() {
    foo<A<T>>();
  }
}
```

```move
module 0x2::n {
  struct A<T> {}

  // Infinitely many types -- NOT allowed.
  // error!
  // foo<T1, T2> -> bar<T2, T1> -> foo<T2, A<T1>>
  //   -> bar<A<T1>, T2> -> foo<A<T1>, A<T2>>
  //   -> bar<A<T2>, A<T1>> -> foo<A<T2>, A<A<T1>>>
  //   -> ...
  fun foo<T1, T2>() {
    bar<T2, T1>();
  }

  fun bar<T1, T2>() {
    foo<T1, A<T2>>();
  }
}
```

Note that the check for type-level recursions is based on a conservative analysis of the call sites and does NOT take control flow or runtime values into account.

```move
module 0x2::m {
  struct A<T> {}

  fun foo<T>(n: u64) {
    if (n > 0) {
      foo<A<T>>(n - 1);
    };
  }
}
```

The function in the example above will technically terminate for any given input and therefore create only finitely many types, but it is still considered invalid by Move's type system.

## Abilities

Abilities are a typing feature in Move that controls what actions are permissible for values of a given type. This system grants fine-grained control over the "linear" typing behavior of values, as well as if and how values are used in global storage. This is implemented by gating access to certain bytecode instructions so that for a value to be used with the bytecode instruction, it must have the ability required (if one is required at all—not every instruction is gated by an ability).

### The Four Abilities

The four abilities are:

- [`copy`](#copy)
  - Allows values of types with this ability to be copied.
- [`drop`](#drop)
  - Allows values of types with this ability to be popped/dropped.
- [`store`](#store)
  - Allows values of types with this ability to exist inside a struct in global storage.
- [`key`](#key)
  - Allows the type to serve as a key for global storage operations.

#### `copy`

The `copy` ability allows values of types with that ability to be copied. It gates the ability to copy values out of local variables with the [`copy`](./variables.md#move-and-copy) operator and to copy values via references with [dereference `*e`](./references.md#reading-and-writing-through-references).

If a value has `copy`, all values contained inside of that value have `copy`.

#### `drop`

The `drop` ability allows values of types with that ability to be dropped. By dropped, we mean that value is not transferred and is effectively destroyed as the Move program executes. As such, this ability gates the ability to ignore values in a multitude of locations, including:

- not using the value in a local variable or parameter
- not using the value in a [sequence via `;`](./variables.md#expression-blocks)
- overwriting values in variables in [assignments](./variables.md#assignments)
- overwriting values via references when [writing `*e1 = e2`](./references.md#reading-and-writing-through-references).

If a value has `drop`, all values contained inside of that value have `drop`.

#### `store`

The `store` ability allows values of types with this ability to exist inside a struct (resource) in global storage, _but_ not necessarily as a top-level resource in global storage. This is the only ability that does not directly gate an operation. Instead, it gates the existence in global storage when used in tandem with `key`.

If a value has `store`, all values contained inside of that value have `store`.

#### `key`

The `key` ability allows the type to serve as a key for [global storage operations](./global-storage.md). It gates all global storage operations, so in order for a type to be used with `move_to`, `borrow_global`, `move_from`, etc., the type must have the `key` ability. Note that the operations still must be used in the module where the `key` type is defined (in a sense, the operations are private to the defining module).

If a value has `key`, all values contained inside of that value have `store`. This is the only ability with this sort of asymmetry.

### Builtin Types

Most primitive, builtin types have `copy`, `drop`, and `store` except for `signer`, which just has `drop`.

- `bool`, `u8`, `u16`, `u32`, `u64`, `u128`, `u256`, and `address` all have `copy`, `drop`, and `store`.
- `signer` has `drop`
  - Cannot be copied and cannot be put into global storage
- `vector<T>` may have `copy`, `drop`, and `store` depending on the abilities of `T`.
  - See [Conditional Abilities and Generic Types](#conditional-abilities-and-generic-types) for more details.
- Immutable references `&` and mutable references `&mut` both have `copy` and `drop`.
  - This refers to copying and dropping the reference itself, not what they refer to.
  - References cannot appear in global storage, hence they do not have `store`.

None of the primitive types have `key`, meaning none of them can be used directly with the [global storage operations](./global-storage.md).

### Annotating Structs

To declare that a `struct` has an ability, it is declared with `has <ability>` after the struct name but before the fields. For example:

```move
module 0x42::example {
  struct Ignorable has drop { f: u64 }

  struct Pair has copy, drop, store { x: u64, y: u64 }
}
```

In this case: `Ignorable` has the `drop` ability. `Pair` has `copy`, `drop`, and `store`.

All of these abilities have strong guarantees over these gated operations. The operation can be performed on the value only if it has that ability; even if the value is deeply nested inside some other collection!

As such: when declaring a struct’s abilities, certain requirements are placed on the fields. All fields must satisfy these constraints. These rules are necessary so that structs satisfy the reachability rules for the abilities given above. If a struct is declared with the ability...

- `copy`, all fields must have `copy`.
- `drop`, all fields must have `drop`.
- `store`, all fields must have `store`.
- `key`, all fields must have `store`.
  - `key` is the only ability currently that doesn't require itself.

For example:

```move
module 0x42::example {
  // A struct without any abilities
  struct NoAbilities {}

  struct WantsCopy has copy {
    f: NoAbilities, // ERROR 'NoAbilities' does not have 'copy'
  }
}
```

and similarly:

```move
module 0x42::example {
  // A struct without any abilities
  struct NoAbilities {}

  struct MyResource has key {
    f: NoAbilities, // Error 'NoAbilities' does not have 'store'
  }
}
```

### Conditional Abilities and Generic Types

When abilities are annotated on a generic type, not all instances of that type are guaranteed to have that ability. Consider this struct declaration:

```move
module 0x42::example {
  struct Cup<T> has copy, drop, store, key { item: T }
}
```

It might be very helpful if `Cup` could hold any type, regardless of its abilities. The type system can _see_ the type parameter, so it should be able to remove abilities from `Cup` if it _sees_ a type parameter that would violate the guarantees for that ability.

This behavior might sound a bit confusing at first, but it might be more understandable if we think about collection types. We could consider the builtin type `vector` to have the following type declaration:

```move
vector<T> has copy, drop, store;
```

We want `vector`s to work with any type. We don't want separate `vector` types for different abilities. So what are the rules we would want? Precisely the same ones we would want for the field rules above. So, it would be safe to copy a `vector` value only if the inner elements can be copied. It would be safe to ignore a `vector` value only if the inner elements can be ignored/dropped. And, it would be safe to put a `vector` in global storage only if the inner elements can be in global storage.

To have this extra expressiveness, a type might not have all the abilities it was declared with, depending on the instantiation of that type; instead, the abilities a type will have depend on both its declaration **and** its type arguments. For any type, type parameters are pessimistically assumed to be used inside the struct, so the abilities are only granted if the type parameters meet the requirements described above for fields. Taking `Cup` from above as an example:

- `Cup` has the ability `copy` only if `T` has `copy`.
- It has `drop` only if `T` has `drop`.
- It has `store` only if `T` has `store`.
- It has `key` only if `T` has `store`.

Here are examples for this conditional system for each ability:

#### Example: conditional `copy`

```move
module 0x42::example {
  struct NoAbilities {}

  struct S has copy, drop { f: bool }

  struct Cup<T> has copy, drop, store { item: T }

  fun example(c_x: Cup<u64>, c_s: Cup<S>) {
    // Valid, 'Cup<u64>' has 'copy' because 'u64' has 'copy'
    let c_x2 = copy c_x;
    // Valid, 'Cup<S>' has 'copy' because 'S' has 'copy'
    let c_s2 = copy c_s;
  }

  fun invalid(c_account: Cup<signer>, c_n: Cup<NoAbilities>) {
    // Invalid, 'Cup<signer>' does not have 'copy'.
    // Even though 'Cup' was declared with copy, the instance does not have 'copy'
    // because 'signer' does not have 'copy'
    let c_account2 = copy c_account;
    // Invalid, 'Cup<NoAbilities>' does not have 'copy'
    // because 'NoAbilities' does not have 'copy'
    let c_n2 = copy c_n;
  }
}
```

#### Example: conditional `drop`

```move
module 0x42::example {
  struct NoAbilities {}

  struct S has copy, drop { f: bool }

  struct Cup<T> has copy, drop, store { item: T }

  fun unused() {
    Cup<bool> { item: true }; // Valid, 'Cup<bool>' has 'drop'
    Cup<S> { item: S { f: false } }; // Valid, 'Cup<S>' has 'drop'
  }

  fun left_in_local(c_account: Cup<signer>): u64 {
    let c_b = Cup<bool> { item: true };
    let c_s = Cup<S> { item: S { f: false } };
    // Valid return: 'c_account', 'c_b', and 'c_s' have values
    // but 'Cup<signer>', 'Cup<bool>', and 'Cup<S>' have 'drop'
    0
  }

  fun invalid_unused() {
    // Invalid, Cannot ignore 'Cup<NoAbilities>' because it does not have 'drop'.
    // Even though 'Cup' was declared with 'drop', the instance does not have 'drop'
    // because 'NoAbilities' does not have 'drop'
    Cup<NoAbilities> { item: NoAbilities {} };
  }

  fun invalid_left_in_local(): u64 {
    let c_n = Cup<NoAbilities> { item: NoAbilities {} };
    // Invalid return: 'c_n' has a value
    // and 'Cup<NoAbilities>' does not have 'drop'
    0
  }
}
```

#### Example: conditional `store`

```move
module 0x42::example {
  struct Cup<T> has copy, drop, store { item: T }

  // 'MyInnerResource' is declared with 'store' so all fields need 'store'
  struct MyInnerResource has store {
    yes: Cup<u64>,
    // Valid, 'Cup<u64>' has 'store'
    // no: Cup<signer>, Invalid, 'Cup<signer>' does not have 'store'
  }

  // 'MyResource' is declared with 'key' so all fields need 'store'
  struct MyResource has key {
    yes: Cup<u64>,
    // Valid, 'Cup<u64>' has 'store'
    inner: Cup<MyInnerResource>,
    // Valid, 'Cup<MyInnerResource>' has 'store'
    // no: Cup<signer>, Invalid, 'Cup<signer>' does not have 'store'
  }
}
```

#### Example: conditional `key`

```move
module 0x42::example {
  struct NoAbilities {}

  struct MyResource<T> has key { f: T }

  fun valid(account: &signer) acquires MyResource {
    let addr = signer::address_of(account);
    // Valid, 'MyResource<u64>' has 'key'
    let has_resource = exists<MyResource<u64>>(addr);
    if (!has_resource) {
      // Valid, 'MyResource<u64>' has 'key'
      move_to(account, MyResource<u64> { f: 0 })
    };
    // Valid, 'MyResource<u64>' has 'key'
    let r = borrow_global_mut<MyResource<u64>>(addr)
    r.f = r.f + 1;
  }

  fun invalid(account: &signer) {
    // Invalid, 'MyResource<NoAbilities>' does not have 'key'
    let has_it = exists<MyResource<NoAbilities>>(addr);
    // Invalid, 'MyResource<NoAbilities>' does not have 'key'
    let NoAbilities {} = move_from<NoAbilities>(addr);
    // Invalid, 'MyResource<NoAbilities>' does not have 'key'
    move_to(account, NoAbilities {});
    // Invalid, 'MyResource<NoAbilities>' does not have 'key'
    borrow_global<NoAbilities>(addr);
  }
}
```
