# Language Versions

The Move 2 language releases are described on this page. The reference documentation of the new features is integrated into the book, and marked in the text with "_Since language version 2.n_".

## Move 2.4

The Move 2.4 language release adds the following features to Move:

- **Struct and Enum Visibility Modifiers**: Structs and enums can now be declared with `public`, `package`, or `friend` visibility modifiers. This allows external modules to construct, destruct, pattern-match, and access or modify fields of the type, lifting the previous restriction that limited all such operations to the defining module. See the reference docs for [structs](./structs-and-enums.md#struct-visibility) and [enums](./structs-and-enums.md#enum-visibility).
- **Compositional Specifications for the Move Prover**: The Move Specification Language gains a set of compositional constructs supporting higher-order functions, frame conditions, intermediate-state reasoning, and proof guidance. See the reference docs for [Behavioral Predicates](./spec-compositional.md#behavioral-predicates), [Access Specifiers and Frame Conditions](./spec-compositional.md#access-specifiers-and-frame-conditions), [State Labels](./spec-compositional.md#state-labels), [Two-State Specification Functions](./spec-compositional.md#two-state-specification-functions), [Proofs and Lemmas](./spec-proofs.md#proofs-and-lemmas), and [Specification Inference](./spec-proofs.md#specification-inference).

## Move 2.3

The Move 2.3 language release adds the following features to Move:

- **Signed Integer Types**: Move now supports `i8`, `i16`, `i32`, `i64`, `i128`, and `i256` signed integer types. See the [reference doc here](./primitive-types.md).
- **Builtin Constants**: Move now supports a number of builtin constants, namely min/max values for integer types (e.g. `MAX_U64`, `MIN_I32`), as well as a constant to determine whether code is running in testing mode. See the [reference doc here](./constants.md#builtin-constants).

## Move 2.2

The Move 2.2 language release adds the following features to Move:

- **Optional Acquires**: The `acquires` annotation on function declarations can be omitted, to be inferred by the compiler.
- **Function Values**: Move now supports function values, which can be passed around as parameters and stored in resources. See the [reference doc here](./functions.md#function-values).
- **Comparison Operations**: Move now supports comparison operations (`<`, `>`, `<=`, `>=`) on all types. See the [reference doc here](./equality-and-comparison.md#typing).

## Move 2.1

The Move 2.1 language release adds the following features to Move:

- **Compound Assignments**: One can now use `x += n`, `x -= n`, etc. to combine assignments and arithmetic operations. See the [reference doc here](./variables.md#compound-assignments) for the supported operations.

- **Loop Labels**: One can now use labels for loops and have a `break` or `continue` expression refer to those labels. This makes it possible to continue or break outer loops from within nested loops. See the [reference doc here](./conditionals-and-loops.md#loop-labels).

- **Underscore function parameters are wildcards, not symbols**: Function parameters named `_` no longer act like variables: they do not bind a value, and multiple such parameters to a function do not cause a conflict. Using `_` in a value expression will yield an error, as it has no value. This makes the behavior of `_` more like the wildcard it is in patterns and `let` expressions, where it does not bind a value.

## Move 2.0

The Move 2.0 language release adds the following features to Move:

- **Enum Types**: add the option to define different variants of data layout in one storable type. They are documented in the [Enum Type section](./structs-and-enums.md).

- **Receiver Style Functions**: add the ability to call functions in the familiar notation `value.func(arg)`. They are documented in [this section](./functions.md#dot-receiver-function-call-style).

- **Index Notation**: allows access to [elements of vectors](./vector.md#index-notation-for-vectors) and of [resource storage](./global-storage.md#index-notation-for-storage-operators) with notations like `&mut vector[index]` or `&mut Resource[addr]`.

- **Positional Structs**: allow defining wrapper types such as `struct Wrapped(u64)`. Positional structs are described [here](./structs-and-enums.md#positional-structs). Enum variants can also be positional.

- **Dot-dot pattern wildcards**: enable statements like `let Struct{x, ..} = value` to match selective parts of data. They are described [here](./structs-and-enums.md#partial-patterns). These patterns are also allowed for enum variants.

- **Package visibility**: allows declaring a function to be visible anywhere inside, but not outside, a package. Friend functions continue to be supported, although package visibility is in many cases more suitable. As a more concise notation, package and friend functions can be declared as `package fun` or `friend fun`, respectively, instead of the longer `public(package) fun` and `public(friend) fun`. This feature is documented [here](./functions.md#package-visibility).

- **Assert abort code optional**: The `assert!` macro can now be used with just one argument, omitting the abort code, in which case a default code will be chosen. See [here](./abort-and-assert.md#assert).

- **New Cast Syntax**: Until now, casts always had to be in parentheses, requiring code like `function((x as u256))`. This requirement is now dropped and casts can be top-level expressions without parentheses, as in `function(x as u256)`. One still needs to write `(x as u64) + (y as u64)` in expressions. This similarly applies to the new enum variant test, `data is VersionedData::V1`.

- **Well-defined evaluation order**: The evaluation order in the cases below is now well-defined (these were previously unspecified):
  - The (a) arguments to a function call and the (b) operand expressions in a binary operation are both evaluated from left to right.
  - Given a "mutate" expression (see [mutating through a reference](./variables.md#mutating-through-a-reference)) of the form `*lexp = rexp`, where `lexp` is an expression of type `&mut T` and `rexp` is an expression of type `T`, `rexp` is evaluated first, followed by `lexp`.

- **Bug fix for acquires annotation**: [A function should be annotated with `acquires`](./functions.md#acquires) if and only if it accesses a resource using `move_from`, `borrow_global`, or `borrow_global_mut`, either directly or transitively through a call. Otherwise, it is an error. Previously, when the transitive call graph included a cycle, such errors were not reported: this was incorrect behavior. We have now corrected this behavior to report these errors even when the transitive call graph has cycles.
