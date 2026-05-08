# Functions

Function syntax in Move is shared between module functions and script functions. Functions inside of modules are reusable, whereas script functions are only used once to invoke a transaction.

## Declaration

Functions are declared with the `fun` keyword followed by the function name, type parameters, parameters, a return type, acquires annotations, and finally the function body.

```text
fun <identifier><[type_parameters: constraint],*>([identifier: type],*): <return_type> <acquires [identifier],*> <function_body>
```

For example

```move
fun foo<T1, T2>(x: u64, y: T1, z: T2): (T2, T1, u64) { (z, y, x) }
```

### Visibility

Module functions, by default, can only be called within the same module. These internal (sometimes called private) functions cannot be called from other modules or from scripts.

```move
address 0x42 {
module m {
    fun foo(): u64 { 0 }

    fun calls_foo(): u64 { foo() } // valid
}

module other {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
        //       ^^^^^ `foo` is internal to `0x42::m`
    }
}
}

script {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
        //       ^^^^^ `foo` is internal to `0x42::m`
    }
}
```

To allow access from other modules or from scripts, the function must be declared `public` or `public(friend)`.

#### `public` visibility

A `public` function can be called by _any_ function defined in _any_ module or script. As shown in the following example, a `public` function can be called by:

- other functions defined in the same module,
- functions defined in another module, or
- the function defined in a script.

There are also no restrictions on what argument types a public function can take or on what its return type can be.

```move
address 0x42 {
module m {
    public fun foo(): u64 { 0 }

    fun calls_foo(): u64 { foo() } // valid
}

module other {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // valid
    }
}
}

script {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // valid
    }
}
```

### `package` visibility

_Since language version 2.0_

A `package` function can only be called within the same package. The notion of a package is
defined by the hosting environment of Move, and not explicit in the language. Typically, the package
is defined by a manifest file `Move.toml` which is processed by the build environment.

The following works, provided the two modules belong to the same package and are at the same address:

```move
module 0x42::m {
  package fun foo(): u64 { 0 }
}

module 0x42::other {
  fun calls_m_foo(): u64 {
    0x42::m::foo() // valid
  }
}
```

An attempt to access `0x42::m::foo` from another package will fail at compile time.

In addition to the notation `package fun`, the longer notation `public(package) fun` is also supported.

Notice that package visibility is a compile-time concept that is reduced by the compiler to friend visibility (described [below](#friend-visibility)), which can be verified by the Move VM. The Move VM guarantees that friend functions
cannot be called across address boundaries, independent of what package system a compilation environment supports.

#### `public(friend)` visibility

_Since language version 2.0_, `friend fun` replaces `public(friend) fun`. The old notation is still supported.

The `public(friend)` visibility modifier is a more restricted form of the `public` modifier to give more control about where a function can be used. A `public(friend)` function can be called by:

- other functions defined in the same module, or
- functions defined in modules which are explicitly specified in the **friend list** (see [Friends](./modules-and-packages.md) on how to specify the friend list), and which reside at the same address.

Note that since we cannot declare a script to be a friend of a module, the functions defined in scripts can never call a `public(friend)` function.

```move
address 0x42 {
module m {
    friend 0x42::n;  // friend declaration
    public(friend) fun foo(): u64 { 0 }
    friend fun foo2(): u64 { 0 } // Since Move 2.0

    fun calls_foo(): u64 { foo() } // valid
    fun calls_foo2(): u64 { foo2() } // valid, since Move 2.0
}

module n {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // valid
    }

    fun calls_m_foo2(): u64 {
        0x42::m::foo2() // valid, since Move 2.0
    }
}

module other {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
        //       ^^^^^ `foo` can only be called from a `friend` of module `0x42::m`
    }

    fun calls_m_foo2(): u64 {
        0x42::m::foo2() // ERROR!
        //       ^^^^^^ `foo2` can only be called from a `friend` of module `0x42::m`
    }
}
}

script {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
        //       ^^^^^ `foo` can only be called from a `friend` of module `0x42::m`
    }
}
```

### `entry` modifier

The `entry` modifier is designed to allow module functions to be safely and directly invoked much like scripts. This allows module writers to specify which functions can be invoked to begin execution. The module writer then knows that any non-`entry` function will be called from a Move program already in execution.

Essentially, `entry` functions are the "main" functions of a module, and they specify where Move programs start executing.

Note, though, that an `entry` function _can_ still be called by other Move functions. So while they _can_ serve as the start of a Move program, they aren't restricted to that case.

For example:

```move
address 0x42 {
module m {
    public entry fun foo() {}

    fun calls_foo() { foo(); } // valid!
}

module n {
    fun calls_m_foo() {
        0x42::m::foo(); // valid!
    }
}

module other {
    public entry fun calls_m_foo() {
        0x42::m::foo(); // valid!
    }
}
}

script {
    fun calls_m_foo() {
        0x42::m::foo(); // valid!
    }
}
```

Even internal functions can be marked as `entry`! This lets you guarantee that the function is called only at the beginning of execution (assuming you do not call it elsewhere in your module)

```move
address 0x42 {
module m {
    entry fun foo() {} // valid! entry functions do not have to be public
}

module n {
    fun calls_m_foo() {
        0x42::m::foo(); // ERROR!
        //       ^^^^^ `foo` is internal to `0x42::m`
    }
}

module other {
    public entry fun calls_m_foo() {
        0x42::m::foo(); // ERROR!
        //       ^^^^^ `foo` is internal to `0x42::m`
    }
}
}

script {
    fun calls_m_foo() {
        0x42::m::foo(); // ERROR!
        //       ^^^^^ `foo` is internal to `0x42::m`
    }
}
```

Entry functions can accept parameters of the following types: primitive types, references to a
`signer`, vectors (where the element type is itself acceptable),
and certain standard library types such as `String`, `Object`, and `Option`.
Entry functions must not have any return values.

_Since language version 2.4_: `public` structs and enums can also be used as entry and view function arguments, provided the type has the `copy` ability and does not have the `key` ability. All field types must themselves be valid argument types, recursively. This requirement on `copy` ensures that no resource type can be constructed from transaction arguments, preserving Move's resource safety guarantees.

### Name

Function names can start with letters `a` to `z` or letters `A` to `Z`. After the first character, function names can contain underscores `_`, letters `a` to `z`, letters `A` to `Z`, or digits `0` to `9`.

```move
module 0x42::example {
    // all valid
    fun FOO() {}

    fun bar_42() {}

    fun bAZ19() {}

    // invalid
    fun _bAZ19() {} // Function names cannot start with '_'
}
```

### Type Parameters

After the name, functions can have type parameters

```move
module 0x42::example {
    fun id<T>(x: T): T { x }

    fun example<T1: copy, T2>(x: T1, y: T2): (T1, T1, T2) { (copy x, x, y) }
}
```

For more details, see [Move generics](./generics-and-abilities.md).

### Parameters

Function parameters are declared with a local variable name followed by a type annotation:

```move
module 0x42::example {
    fun add(x: u64, y: u64): u64 { x + y }
}
```

We read this as `x` has type `u64`

A function does not have to have any parameters at all.

```move
module 0x42::example {
    fun useless() {}
}
```

This is very common for functions that create new or empty data structures

```move
module 0x42::example {
    struct Counter { count: u64 }

    fun new_counter(): Counter {
        Counter { count: 0 }
    }
}
```

### Acquires

When a function accesses a resource using `move_from`, `borrow_global`, or `borrow_global_mut`, the function must indicate that it `acquires` that resource. This is then used by Move's type system to ensure the references into global storage are safe, specifically that there are no dangling references into global storage.

```move
module 0x42::example {

    struct Balance has key { value: u64 }

    public fun add_balance(s: &signer, value: u64) {
        move_to(s, Balance { value })
    }

    public fun extract_balance(addr: address): u64 acquires Balance {
        let Balance { value } = move_from<Balance>(addr); // acquires needed
        value
    }
}
```

`acquires` annotations must also be added for transitive calls within the module. Calls to these functions from another module do not need to be annotated with these acquires because one module cannot access resources declared in another module — so the annotation is not needed to ensure reference safety.

```move
module 0x42::example {

    struct Balance has key { value: u64 }

    public fun add_balance(s: &signer, value: u64) {
        move_to(s, Balance { value })
    }

    public fun extract_balance(addr: address): u64 acquires Balance {
        let Balance { value } = move_from<Balance>(addr); // acquires needed
        value
    }

    public fun extract_and_add(sender: address, receiver: &signer) acquires Balance {
        let value = extract_balance(sender); // acquires needed here
        add_balance(receiver, value)
    }
}

module 0x42::other {
    fun extract_balance(addr: address): u64 {
        0x42::example::extract_balance(addr) // no acquires needed
    }
}
```

A function can `acquire` as many resources as it needs to

```move
module 0x42::example {
    use std::vector;

    struct Balance has key { value: u64 }

    struct Box<T> has key { items: vector<T> }

    public fun store_two<Item1: store, Item2: store>(
        addr: address,
        item1: Item1,
        item2: Item2,
    ) acquires Balance, Box {
        let balance = borrow_global_mut<Balance>(addr); // acquires needed
        balance.value = balance.value - 2;
        let box1 = borrow_global_mut<Box<Item1>>(addr); // acquires needed
        vector::push_back(&mut box1.items, item1);
        let box2 = borrow_global_mut<Box<Item2>>(addr); // acquires needed
        vector::push_back(&mut box2.items, item2);
    }
}
```

### Return type

After the parameters, a function specifies its return type.

```move
module 0x42::example {
    fun zero(): u64 { 0 }
}
```

Here `: u64` indicates that the function's return type is `u64`.

> **Note:** A function can return an immutable `&` or mutable `&mut` [reference](./references.md) if derived from an input reference. Keep in mind that this means a function [cannot return a reference to global storage](./references.md#references-cannot-be-stored) unless it is an [inline function](#inline-functions).

Using tuples, a function can return multiple values:

```move
module 0x42::example {
    fun one_two_three(): (u64, u64, u64) { (0, 1, 2) }
}
```

If no return type is specified, the function has an implicit return type of unit `()`. These functions are equivalent:

```move
module 0x42::example {
    fun just_unit1(): () { () }

    fun just_unit2() { () }

    fun just_unit3() {}
}
```

`script` functions must have a return type of unit `()`:

```move
script {
    fun do_nothing() {}
}
```

As mentioned in the [tuples section](./tuples.md), these tuple "values" are virtual and do not exist at runtime. So for a function that returns unit `()`, it will not be returning any value at all during execution.

### Function body

A function's body is an expression block. The return value of the function is the last value in the sequence

```move
module 0x42::example {
    fun example(): u64 {
        let x = 0;
        x = x + 1;
        x // returns 'x'
    }
}
```

See [the section below for more information on returns](#returning-values)

For more information on expression blocks, see [Move variables](./variables.md).

### Native Functions

Some functions do not have a body specified, and instead have the body provided by the VM. These functions are marked `native`.

Without modifying the VM source code, a programmer cannot add new native functions. Furthermore, it is the intent that `native` functions are used for either standard library code or for functionality needed for the given Move environment.

Most `native` functions you will likely see are in standard library code such as `vector`

```move
module std::vector {
    native public fun empty<Element>(): vector<Element>;
    // ...
}
```

## Calling

When calling a function, the name can be specified either through an alias or fully qualified

```move
module 0x42::example {
    public fun zero(): u64 { 0 }
}

script {
    use 0x42::example::{Self, zero};

    fun call_zero() {
        // With the `use` above all of these calls are equivalent
        0x42::example::zero();
        example::zero();
        zero();
    }
}
```

When calling a function, an argument must be given for every parameter.

```move
module 0x42::example {
    public fun takes_none(): u64 { 0 }

    public fun takes_one(x: u64): u64 { x }

    public fun takes_two(x: u64, y: u64): u64 { x + y }

    public fun takes_three(x: u64, y: u64, z: u64): u64 { x + y + z }
}

script {
    use 0x42::example;

    fun call_all() {
        example::takes_none();
        example::takes_one(0);
        example::takes_two(0, 1);
        example::takes_three(0, 1, 2);
    }
}
```

Type arguments can be either specified or inferred. Both calls are equivalent.

```move
module 0x42::example {
    public fun id<T>(x: T): T { x }
}

script {
    use 0x42::example;

    fun call_all() {
        example::id(0);
        example::id<u64>(0);
    }
}
```

For more details, see [Move generics](./generics-and-abilities.md).

## Returning values

The result of a function, its "return value", is the final value of its function body. For example

```move
module 0x42::example {
    fun add(x: u64, y: u64): u64 {
        x + y
    }
}
```

[As mentioned above](#function-body), the function's body is an [expression block](./variables.md). The expression block can be a sequence of various statements, and the final expression in the block will be the value of that block.

```move
module 0x42::example {
    fun double_and_add(x: u64, y: u64): u64 {
        let double_x = x * 2;
        let double_y = y * 2;
        double_x + double_y
    }
}
```

The return value here is `double_x + double_y`

### `return` expression

A function implicitly returns the value that its body evaluates to. However, functions can also use the explicit `return` expression:

```move
module 0x42::example {
    fun f1(): u64 { return 0 }

    fun f2(): u64 { 0 }
}
```

These two functions are equivalent. In this slightly more involved example, the function subtracts two `u64` values, but returns early with `0` if the second value is too large:

```move
module 0x42::example {
    fun safe_sub(x: u64, y: u64): u64 {
        if (y > x) return 0;
        x - y
    }
}
```

Note that the body of this function could also have been written as `if (y > x) 0 else x - y`.

However, where `return` really shines is in exiting deep within other control flow constructs. In this example, the function iterates through a vector to find the index of a given value:

```move
module 0x42::example {
    use std::vector;
    use std::option::{Self, Option};

    fun index_of<T>(v: &vector<T>, target: &T): Option<u64> {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            if (vector::borrow(v, i) == target) return option::some(i);
            i = i + 1
        };

        option::none()
    }
}
```

Using `return` without an argument is shorthand for `return ()`. That is, the following two functions are equivalent:

```move
module 0x42::example {
    fun foo1() { return }

    fun foo2() { return () }
}
```

## Inline Functions

Inline functions are functions whose bodies are expanded in place at the caller location during compile time.
Thus, inline functions do not appear in Move bytecode as separate functions: all calls to them are expanded away by the compiler.
In certain circumstances, they may lead to faster execution and save gas.
However, users should be aware that they could lead to larger bytecode size: excessive inlining potentially triggers various size restrictions.

One can define an inline function by adding the `inline` keyword to a function declaration as shown below:

```move
module 0x42::example {
    inline fun percent(x: u64, y: u64): u64 { x * 100 / y }
}
```

If we call this inline function as `percent(2, 200)`, the compiler will replace this call with the inline function's body, as if the user has written `2 * 100 / 200`.

### Function parameters and lambda expressions

Inline functions support _function parameters_, which accept lambda expressions (i.e., anonymous functions) as arguments.
This feature allows writing several common programming patterns elegantly.
Similar to inline functions, lambda expressions are also expanded at call site.

A lambda expression includes a list of parameter names (enclosed within `||`) followed by the body.
Some simple examples are: `|x| x + 1`, `|x, y| x + y`, `|| 1`, `|| { 1 }`.
A lambda's body can refer to variables available in the scope where the lambda is defined: this is also known as capturing.
Such variables can be read or written (if mutable) by the lambda expression.

The type of function parameter is written as `|<list of parameter types>| <return type>`.
For example, when the function parameter type is `|u64, u64| bool`, any lambda expression that takes two `u64` parameters and returns a `bool` value can be provided as the argument.

Below is an example that showcases many of these concepts in action (this example is taken from the `std::vector` module):

```move
module 0x42::example {
    /// Fold the function over the elements.
    /// E.g, `fold(vector[1,2,3], 0, f)` is the same as `f(f(f(0, 1), 2), 3)`.
    public inline fun fold<Accumulator, Element>(
        v: vector<Element>,
        init: Accumulator,
        f: |Accumulator, Element|Accumulator
    ): Accumulator {
        let accu = init;
        // Note: `for_each` is an inline function, but is not shown here.
        for_each(v, |elem| accu = f(accu, elem));
        accu
    }
}
```

The type signature of the elided public inline function `for_each` is `fun for_each<Element>(v: vector<Element>, f: |Element|)`.
Its second parameter `f` is a function parameter which accepts any lambda expression that consumes an `Element` and returns nothing.
In the code example, we use the lambda expression `|elem| accu = f(accu, elem)` as an argument to this function parameter.
Note that this lambda expression captures the variable `accu` from the outer scope.

### Current restrictions

There are plans to loosen some of these restrictions in the future, but for now,

- Only inline functions can have function parameters.
- Only explicit lambda expressions can be passed as an argument to an inline function's function parameters.
- Inline functions and lambda expressions
  - cannot have `return` expressions; or free `break` or `continue` expressions (occurring outside of a loop)
  - cannot return lambda expressions.
- Cyclic recursion involving only inline functions is not allowed.
- Parameters in lambda expressions must not be type annotated (e.g., `|x: u64| x + 1` is not allowed): their types are inferred.

### Additional considerations

- Avoid using module-private constants/methods in public inline functions.
  When such inline functions are called outside of that module, an in-place expansion at call site leads to invalid access of the private constants/methods.
- Avoid marking large functions that are called at different locations as inline. Also avoid inline functions calling lots of other inline functions transitively.
  These may lead to excessive inlining and increase the bytecode size.
- Inline functions can be useful for returning references to global storage, which non-inline functions cannot do.

### Inline functions and references

As mentioned briefly [in a "tip" above](#return-type) `inline` functions can use references more freely than normal functions.

For example, actual arguments to a call to a non-`inline` function may not be aliased unsafely
(multiple `&` parameters referring to the same object, with at least one of them `&mut`),
but calls to `inline` functions do not necessarily have that restriction, as long as no reference
usage conflicts remain after the function is inlined.

```move
inline fun add(dest: &mut u64, a: &u64, b: &u64) {
    *dest = *a + *b;
}

fun user(...) {
    ...
    x = 3;
    add(&mut x, &x, &x);  // legal only because of inlining
    ...
}
```

A reference-typed value returned from a non-inline function must be derived from a reference parameter
passed to the function, but this need not be the case for an inline function, as long as the referred
value is in the function scope after inlining.

The exact details of reference safety and "borrow checking" are complex and documented elsewhere.
Advanced Move users find new expressiveness by understanding that
"borrow checking" happens only after all `inline` function calls are expanded.

However, with this power comes new responsibility: documentation of a nontrivial `inline` function should
probably explain any underlying restrictions on reference parameters and results at a call site.

## Dot (receiver) function call style

_Since language version 2.0_

By using the well-known name `self` as the first parameter for a function declaration, one can enable calling this function with the `.` syntax -- often also called receiver style syntax. Example:

```move
module 0x42::example {
    struct S {}

    fun foo(self: &S, x: u64) { /* ... */ }

    //...

    fun example() {
        let s = S {};
        s.foo(1);
    }
}
```

The call `s.foo(1)` is syntactic sugar for `foo(&s, 1)`. Notice that the compiler automatically inserts the reference operator. The 2nd, old notation is still available for `foo`, so one can incrementally introduce the new call style without breaking existing code.

The type of the `self` argument can be a struct or an immutable or mutable reference to a struct. The struct must be declared in the same module as the function.

Notice that you do not need to `use` the modules which introduce receiver functions. The compiler will find those functions automatically based on the argument type of `s` in a call like `s.foo(1)`. This, in combination with the automatic insertion of reference operators, can make code using this syntax significantly more concise.

The receiver style syntax can also be used on generic functions, like shown below for the generic function `std::vector::remove<T>(self: &mut vector<T>, i: u64): T`.

```move
module 0x42::example {
   fun bar() {
       let v = vector[1, 2, 3];
       let e1 = v.remove(0); // type params inferred for `remove<T>`
       assert!(e1 == 1);
       let e2 = v.remove::<u8>(0); // type params explicitly specified
       assert!(e2 == 2);
   }
}
```

## Function Values

_Since language version 2.2_

Move supports _function values_ as first-class citizens of the language. A function value is constructed from the name of a function or by a lambda expression, and is evaluated by passing parameters to it and causing the underlying function to be executed. This feature is often called _dynamic dispatch_. Which concrete function is called is not known to the caller and is determined by the runtime value. Dynamic dispatch is an important tool for composing applications. Move makes dynamic dispatch safe by providing builtin protection mechanisms against reentrancy, which can be further refined by user choice.

### Function Types

The type of function values is already known from [inline functions](#function-parameters-and-lambda-expressions). A function type is denoted, for example, as `|u64|bool`, indicating a function that takes a number and returns a boolean. Lists of types are separated by commas, as in `|u64, bool|(bool, u8)`.

Function types can have associated abilities, written as `|u64|bool has copy`. Multiple abilities are separated by plus, as in `|u64|bool has copy+drop`. If no abilities are provided, the value can be only moved around and evaluated (for evaluation of function values, see [below](#function-evaluation)).

Function values can be stored in fields of structs or enums. In this case, the field type inherits the abilities of the struct:

```move
struct S has key {
  func: |u64| bool /* has store */  // not needed since inherited
}
```

### Operations on Functions

A function value is evaluated by providing the corresponding number of parameters, similarly to calling a named function. During evaluation, the function value is _consumed_. Hence if the value needs to be evaluated multiple times, its type must have the `copy` ability:

```move
let f: |u64|bool has copy = |x| x > 0;
assert!(f(1) == f(2))
```

Function values support equality and ordering. Note that those relations are based on the name of the underlying function behind a runtime value, and do not reflect semantic equivalence.

### Function Type Wrappers

Function types, especially those with abilities, can be verbose, and repetitive when the same function type appears many times in the code. For this purpose, Move recognizes struct wrappers around function types as a special case. They can be used to effectively create named function types:

```move
struct Predicate<T>(|&T|bool) has copy;
```

Move supports this feature by automatically converting function values into the wrapper type and vice versa. Examples:

```move
let f: Predicate<u64> = |x| *x > 0; // lambda converts to Predicate
assert!(f(&22)) // Predicate callable
```

### Denoting Function Values

Function values can be constructed by directly using a function name. The resulting function type is derived from the signature of the underlying function, with abilities `copy+drop`. If the function is public, those function values have the `store` ability as well:

```move
public fun is_even(x: u64): bool { x % 2 == 0 }
fun is_odd(x: u64): bool { x % 2 == 1 }
...
let f: |u64|bool has copy+drop+store = is_even;
let g: |u64|bool has copy+drop = is_odd;
```

A _persistent_ function is required to build a storable function value because it needs to be guaranteed that the underlying function exists and can be safely restored from storage at any point in the future. However, a code upgrade may change the underlying implementation of the function, while its signature is persistent.

While `public` and `entry` functions are persistent by default, a non-public function needs to be marked with the attribute `#[persistent]` to become storable:

```move
#[persistent] fun is_odd(x: u64): bool { x % 2 == 1 }
...
let g: |u64|bool has copy+drop+store = is_odd;
```

Using the `#[persistent]` attribute is preferred if the only objective is to make a function storable, avoiding security implications with public or entry visibility.

### Lambda Expressions and Closures

Function values can be denoted by _lambda expressions_ (as also available as parameters for [inline functions](#function-parameters-and-lambda-expressions)). Lambda expressions can capture context variables _by value_: those values are moved (or copied) into a _closure_, from where they are produced when the function is evaluated. Examples:

```move
struct S(u64); // cannot be copied or dropped
...
let s = S(1);
let add = |y| { let S(x) = s; x + y }; // s will be moved into the closure
assert!(add(2) == 3)
```

Closures with captured values are lexicographically ordered using first the name of the underlying function (which may be generated from lambda lifting), and then the captured values.

The type of the closure constructed by a lambda expression is inferred from the expression (for example, the type of `add` in the example above is inferred as `|u64|u64`). The abilities of this function type are derived as follows. By default, the function underlying a closure is a private function, so the function itself is `copy+drop` (and not `store`). This is intersected with the abilities of all the captured context variables. However, there is a special case for lambdas where instead of a private function an underlying persistent function can be identified, such that the lambda just 'delays' certain arguments of this function. This pattern is also called 'currying' in functional programming (named after the mathematician Curry). Here are some examples:

```move
#[persistent] fun add(x: u64, y: u64) { x + y }
...
let x = 22;
let f: |u64|u64 has copy+drop+store = |y| add(x, y);  // 1st argument captured, 2nd argument delayed
let f: |u64|u64 has copy+drop+store = |y| add(y, x);  // 1st argument delayed, 2nd argument captured
```

Notice it is not possible to _capture_ reference values at this point of time in Move. Thus, the following code does not compile:

```move
let x = &22;
let f = |y| add(*x, y) // DOES NOT COMPILE
```

Related, it is not possible to mutate any locals in the context of a lambda. Specifically, the following pattern as known from lambdas with inline functions, is not supported:

```move
let x = 0;
collection.for_each(|e| x += e) // DOES NOT COMPILE
```

However, the actual parameters of lambdas can be references, only captured values are restricted. For example:

```move
let x = 22;
let f : |&u64|u64 = |y| add(x, *y)
```

### Reentrancy Check

Via dynamic dispatch of function values, reentrancy of modules in a chain of function calls is possible. If module `m1` uses module `m2`, and `m1` calls `m2::f` passing a function value to it, this function value can callback into `m1`. This situation is called _reentrancy_, and is not possible in Move without function values, since the module usage relation is acyclic.

The Move VM dynamically detects reentrancy of a module and _locks_ all resources declared in that module from being accessed. Thus, during reentrancy of `m`, calling resource operations like `&m::R[addr]`, `&mut m::R[addr]`, and `move_from<m::R>` leads to an abort. Here is an example:

```move
module 0x42::caller {
  use 0x42::callee;
  struct R{ count: u64 } has key;
  fun calling() acquires R {
     let r = &mut R[@addr];
     // This callback is OK, because `R` is not accessed
     callee::call_me(r, |x| do_something(x))
     // This callback will lead to reentrancy runtime error
     callee::call_me(r, |_| R[@addr].count += 1)
     r.count += 1
  }
  fun do_something(r: &mut R) { .. }
}

module 0x42::callee {
  fun call_me<T(x: &mut T, action: |&mut T|) {
    action(x)
  }
}
```

Notice that dispatching a function value to a concrete function in the same module is also considered to be reentrancy. If the function `callee::call_me` would be moved into the module `caller`, the same semantics is in effect.

The default reentrancy check ensures consistency of Move's reference semantics and suppresses side effects of reentrancy for the resources owned by the re-entered module. However, re-entered code is allowed to still access resource state managed by modules outside the reentrancy path. Such state accesses can be considered bad design, but they exist.
For these purposes, the `#[module_lock]` attribute can be attached to a function:

```move
module 0x42::account { ... }
module 0x42::caller {
  #[module_lock] // without this lock, the notify call could withdraw more than intended.
  fun transfer(from: address, to: address, amount: u64, notify: |u64|) {
    // Oops. This should be really differently designed, using `Coin` type and moving it.
    assert!(account::balance(from) - MIN_BALANCE >= amount);
    account::deposit(to, amount)
    notify(amount); // attempt to re-enter `transfer` is blocked
    account::withdraw(from, amount);
  }
}
```

While a function with this attribute is running, all calls reentering any module will lead to an abort, providing stronger protection.

The `#[module_lock]` restriction is not the default behavior because it is too strong for typical patterns of higher-order programming. For example, `collection.find(|x| cond(x))` will lead to a reentrancy of the module that contains this expression, from the module that defines the collection type.
