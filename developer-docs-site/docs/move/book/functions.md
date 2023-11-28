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

```move=
address 0x42 {
module m {
    fun foo(): u64 { 0 }
    fun calls_foo(): u64 { foo() } // valid
}

module other {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
//      ^^^^^^^^^^^^ 'foo' is internal to '0x42::m'
    }
}
}

script {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
//      ^^^^^^^^^^^^ 'foo' is internal to '0x42::m'
    }
}
```

To allow access from other modules or from scripts, the function must be declared `public` or `public(friend)`.

#### `public` visibility

A `public` function can be called by *any* function defined in *any* module or script. As shown in the following example, a `public` function can be called by:
- other functions defined in the same module,
- functions defined in another module, or
- the function defined in a script.

There are also no restrictions for what the argument types a public function can take and its return type.

```move=
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

#### `public(friend)` visibility

The `public(friend)` visibility modifier is a more restricted form of the `public` modifier to give more control about where a function can be used. A `public(friend)` function can be called by:
- other functions defined in the same module, or
- functions defined in modules which are explicitly specified in the **friend list** (see [Friends](./friends.md) on how to specify the friend list).

Note that since we cannot declare a script to be a friend of a module, the functions defined in scripts can never call a `public(friend)` function.

```move=
address 0x42 {
module m {
    friend 0x42::n;  // friend declaration
    public(friend) fun foo(): u64 { 0 }
    fun calls_foo(): u64 { foo() } // valid
}

module n {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // valid
    }
}

module other {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
//      ^^^^^^^^^^^^ 'foo' can only be called from a 'friend' of module '0x42::m'
    }
}
}

script {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
//      ^^^^^^^^^^^^ 'foo' can only be called from a 'friend' of module '0x42::m'
    }
}
```

### `entry` modifier

The `entry` modifier is designed to allow module functions to be safely and directly invoked much like scripts. This allows module writers to specify which functions can be to begin execution. The module writer then knows that any non-`entry` function will be called from a Move program already in execution.

Essentially, `entry` functions are the "main" functions of a module, and they specify where Move programs start executing.

Note though, an `entry` function _can_ still be called by other Move functions. So while they _can_ serve as the start of a Move program, they aren't restricted to that case.

For example:

```move=
address 0x42 {
module m {
    public entry fun foo(): u64 { 0 }
    fun calls_foo(): u64 { foo() } // valid!
}

module n {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // valid!
    }
}

module other {
    public entry fun calls_m_foo(): u64 {
        0x42::m::foo() // valid!
    }
}
}

script {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // valid!
    }
}
```

Even internal functions can be marked as `entry`! This lets you guarantee that the function is called only at the beginning of execution (assuming you do not call it elsewhere in your module)

```move=
address 0x42 {
module m {
    entry fun foo(): u64 { 0 } // valid! entry functions do not have to be public
}

module n {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
//      ^^^^^^^^^^^^ 'foo' is internal to '0x42::m'
    }
}

module other {
    public entry fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
//      ^^^^^^^^^^^^ 'foo' is internal to '0x42::m'
    }
}
}

script {
    fun calls_m_foo(): u64 {
        0x42::m::foo() // ERROR!
//      ^^^^^^^^^^^^ 'foo' is internal to '0x42::m'
    }
}
```

Entry functions can take primitive types, String, and vector arguments but cannot take Structs (e.g. Option). They also
must not have any return values.

### Name

Function names can start with letters `a` to `z` or letters `A` to `Z`. After the first character, function names can contain underscores `_`, letters `a` to `z`, letters `A` to `Z`, or digits `0` to `9`.

```move
fun FOO() {}
fun bar_42() {}
fun _bAZ19() {}
```

### Type Parameters

After the name, functions can have type parameters

```move
fun id<T>(x: T): T { x }
fun example<T1: copy, T2>(x: T1, y: T2): (T1, T1, T2) { (copy x, x, y) }
```

For more details, see [Move generics](./generics.md).

### Parameters

Functions parameters are declared with a local variable name followed by a type annotation

```move
fun add(x: u64, y: u64): u64 { x + y }
```

We read this as `x` has type `u64`

A function does not have to have any parameters at all.

```move
fun useless() { }
```

This is very common for functions that create new or empty data structures

```move=
address 0x42 {
module example {
  struct Counter { count: u64 }

  fun new_counter(): Counter {
      Counter { count: 0 }
  }

}
}
```

### Acquires

When a function accesses a resource using `move_from`, `borrow_global`, or `borrow_global_mut`, the function must indicate that it `acquires` that resource. This is then used by Move's type system to ensure the references into global storage are safe, specifically that there are no dangling references into global storage.

```move=
address 0x42 {
module example {

    struct Balance has key { value: u64 }

    public fun add_balance(s: &signer, value: u64) {
        move_to(s, Balance { value })
    }

    public fun extract_balance(addr: address): u64 acquires Balance {
        let Balance { value } = move_from(addr); // acquires needed
        value
    }
}
}
```

`acquires` annotations must also be added for transitive calls within the module. Calls to these functions from another module do not need to annotated with these acquires because one module cannot access resources declared in another module--so the annotation is not needed to ensure reference safety.

```move=
address 0x42 {
module example {

    struct Balance has key { value: u64 }

    public fun add_balance(s: &signer, value: u64) {
        move_to(s, Balance { value })
    }

    public fun extract_balance(addr: address): u64 acquires Balance {
        let Balance { value } = move_from(addr); // acquires needed
        value
    }

    public fun extract_and_add(sender: address, receiver: &signer) acquires Balance {
        let value = extract_balance(sender); // acquires needed here
        add_balance(receiver, value)
    }
}
}

address 0x42 {
module other {
    fun extract_balance(addr: address): u64 {
        0x42::example::extract_balance(addr) // no acquires needed
    }
}
}
```

A function can `acquire` as many resources as it needs to

```move=
address 0x42 {
module example {
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
}
```

### Return type

After the parameters, a function specifies its return type.

```move
fun zero(): u64 { 0 }
```

Here `: u64` indicates that the function's return type is `u64`.

:::tip
A function can return an immutable `&` or mutable `&mut` [reference](./references.md) if derived from an input reference. Keep in mind, this means that a function [cannot return a reference to global storage](./references.md#references-cannot-be-stored) unless it is an [inline function](#inline-functions).
:::

Using tuples, a function can return multiple values:

```move
fun one_two_three(): (u64, u64, u64) { (0, 1, 2) }
```

If no return type is specified, the function has an implicit return type of unit `()`. These functions are equivalent:

```move
fun just_unit(): () { () }
fun just_unit() { () }
fun just_unit() { }
```

`script` functions must have a return type of unit `()`:

```move
script {
    fun do_nothing() {
    }
}
```

As mentioned in the [tuples section](./tuples.md), these tuple "values" are virtual and do not exist at runtime. So for a function that returns unit `()`, it will not be returning any value at all during execution.

### Function body

A function's body is an expression block. The return value of the function is the last value in the sequence

```move=
fun example(): u64 {
    let x = 0;
    x = x + 1;
    x // returns 'x'
}
```

See [the section below for more information on returns](#returning-values)

For more information on expression blocks, see [Move variables](./variables.md).

### Native Functions

Some functions do not have a body specified, and instead have the body provided by the VM. These functions are marked `native`.

Without modifying the VM source code, a programmer cannot add new native functions. Furthermore, it is the intent that `native` functions are used for either standard library code or for functionality needed for the given Move environment.

Most `native` functions you will likely see are in standard library code such as `vector`

```move=
module std::vector {
    native public fun empty<Element>(): vector<Element>;
    ...
}
```

## Calling

When calling a function, the name can be specified either through an alias or fully qualified

```move=
address 0x42 {
module example {
    public fun zero(): u64 { 0 }
}
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

```move=
address 0x42 {
module example {
    public fun takes_none(): u64 { 0 }
    public fun takes_one(x: u64): u64 { x }
    public fun takes_two(x: u64, y: u64): u64 { x + y }
    public fun takes_three(x: u64, y: u64, z: u64): u64 { x + y + z }
}
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

```move=
address 0x42 {
module example {
    public fun id<T>(x: T): T { x }
}
}

script {
    use 0x42::example;
    fun call_all() {
        example::id(0);
        example::id<u64>(0);
    }
}
```

For more details, see [Move generics](./generics.md).


## Returning values

The result of a function, its "return value", is the final value of its function body. For example

```move=
fun add(x: u64, y: u64): u64 {
    x + y
}
```

[As mentioned above](#function-body), the function's body is an [expression block](./variables.md). The expression block can sequence various statements, and the final expression in the block will be the value of that block

```move=
fun double_and_add(x: u64, y: u64): u64 {
    let double_x = x * 2;
    let double_y = y * 2;
    double_x + double_y
}
```

The return value here is `double_x + double_y`

### `return` expression

A function implicitly returns the value that its body evaluates to. However, functions can also use the explicit `return` expression:

```move
fun f1(): u64 { return 0 }
fun f2(): u64 { 0 }
```

These two functions are equivalent. In this slightly more involved example, the function subtracts two `u64` values, but returns early with `0` if the second value is too large:

```move=
fun safe_sub(x: u64, y: u64): u64 {
    if (y > x) return 0;
    x - y
}
```

Note that the body of this function could also have been written as `if (y > x) 0 else x - y`.

However `return` really shines is in exiting deep within other control flow constructs. In this example, the function iterates through a vector to find the index of a given value:

```move=
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
```

Using `return` without an argument is shorthand for `return ()`. That is, the following two functions are equivalent:

```move
fun foo() { return }
fun foo() { return () }
```

## Inline Functions

Inline functions are functions whose bodies are expanded in place at the caller location during compile time.
Thus, inline functions do not appear in Move bytecode as a separate functions: all calls to them are expanded away by the compiler.
In certain circumstances, they may lead to faster execution and save gas.
However, users should be aware that they could lead to larger bytecode size: excessive inlining potentially triggers various size restrictions.

One can define an inline function by adding the `inline` keyword to a function declaration as shown below:

```move=
inline fun percent(x: u64, y: u64):u64 { x * 100 / y }
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

The type of a function parameter is written as `|<list of parameter types>| <return type>`.
For example, when the function parameter type is `|u64, u64| bool`, any lambda expression that takes two `u64` parameters and returns a `bool` value can be provided as the argument.

Below is an example that showcases many of these concepts in action (this example is taken from the `std::vector` module):

```move=
/// Fold the function over the elements. 
/// E.g, `fold(vector[1,2,3], 0, f)` is the same as `f(f(f(0, 1), 2), 3)`.
public inline fun fold<Accumulator, Element>(
    v: vector<Element>,
    init: Accumulator,
    f: |Accumulator,Element|Accumulator
): Accumulator {
  let accu = init;
  // Note: `for_each` is an inline function, but is not shown here.
  for_each(v, |elem| accu = f(accu, elem));
  accu
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
- Inline functions and lambda expressions cannot have `return`, `break`, or `continue` expressions.
- Inline functions or lambda expressions cannot return lambda expressions.
- Cyclic recursion involving only inline functions is not allowed.
- Parameters in lambda expressions must not be type annotated (e.g., `|x: u64| x + 1` is not allowed): their types are inferred.

### Additional considerations

- Avoid using module-private constants/methods in public inline functions.
  When such inline functions are called outside of that module, an in-place expansion at call site leads to invalid access of the private constants/methods.
- Avoid marking large functions that are called at different locations as inline. Also avoid inline functions calling lots of other inline functions transitively.
  These may lead to excessive inlining and increase the bytecode size.
- Inline functions can be useful for returning references to global storage, which non-inline functions cannot do.
