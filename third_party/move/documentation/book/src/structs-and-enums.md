# Structs, Resources, and Enums

## Structs and Resources

A _struct_ is a user-defined data structure containing typed fields. Structs can store any
non-reference type, including other structs.

We often refer to struct values as _resources_ if they cannot be copied and cannot be dropped. In
this case, resource values must have ownership transferred by the end of the function. This property
makes resources particularly well suited for defining global storage schemas or for representing
important values (such as a token).

By default, structs are linear and ephemeral. By this we mean that they: cannot be copied, cannot be
dropped, and cannot be stored in global storage. This means that all values have to have ownership
transferred (linear) and the values must be dealt with by the end of the program's execution
(ephemeral). We can relax this behavior by giving the struct [abilities](./generics-and-abilities.md) which allow
values to be copied or dropped and also to be stored in global storage or to define global storage
schemas.

### Defining Structs

Structs must be defined inside a module:

```move
module 0x2::m {
    struct Foo { x: u64, y: bool }
    struct Bar {}
    struct Baz { foo: Foo, }
    //                   ^ note: it is fine to have a trailing comma
}
```

Structs cannot be recursive, so the following definition is invalid:

```move
module 0x2::m {
  struct Foo { x: Foo }
  //              ^ error! Foo cannot contain Foo
}
```

For positional structs that use numbered instead of named fields, see
the [positional structs](#positional-structs) section.

As mentioned above: by default, a struct declaration is linear and ephemeral. So to allow the value
to be used with certain operations (that copy it, drop it, store it in global storage, or use it as
a storage schema), structs can be granted [abilities](./generics-and-abilities.md) by annotating them with
`has <ability>`:

```move
module 0x2::m {
  struct Foo has copy, drop { x: u64, y: bool }
}
```

For more details, see the [annotating structs](./generics-and-abilities.md#annotating-structs) section.

#### Naming

Structs must start with a capital letter `A` to `Z`. After the first letter, struct names can
contain underscores `_`, letters `a` to `z`, letters `A` to `Z`, or digits `0` to `9`.

```move
module 0x2::m {
  struct Foo {}
  struct BAR {}
  struct B_a_z_4_2 {}
}
```

This naming restriction of starting with `A` to `Z` is in place to give room for future language
features. It may or may not be removed later.

### Using Structs

#### Creating Structs

Values of a struct type can be created (or "packed") by indicating the struct name, followed by
a value for each field:

```move
module 0x2::m {
  struct Foo has drop { x: u64, y: bool }
  struct Baz has drop { foo: Foo }

  fun example() {
    let foo = Foo { x: 0, y: false };
    let baz = Baz { foo };
  }
}
```

If you initialize a struct field with a local variable whose name is the same as the field, you can
use the following shorthand:

```move
module 0x2::m {
  fun example() {
    let baz = Baz { foo: foo };
    // is equivalent to
    let baz = Baz { foo };
  }
}
```

This is sometimes called "field name punning".

#### Destroying Structs via Pattern Matching

Struct values can be destroyed by pattern matching them in `let` bindings or assignments.

```move
module 0x2::m {
  struct Foo { x: u64, y: bool }
  struct Bar { foo: Foo }
  struct Baz {}

  fun example_destroy_foo() {
    let foo = Foo { x: 3, y: false };
    let Foo { x, y: foo_y } = foo;
    //        ^ shorthand for `x: x`

    // two new bindings
    //   x: u64 = 3
    //   foo_y: bool = false
  }

  fun example_destroy_foo_wildcard() {
    let foo = Foo { x: 3, y: false };
    let Foo { x, y: _ } = foo;

    // only one new binding since y was bound to a wildcard
    //   x: u64 = 3
  }

  fun example_destroy_foo_assignment() {
    let x: u64;
    let y: bool;
    Foo { x, y } = Foo { x: 3, y: false };

    // mutating existing variables x & y
    //   x = 3, y = false
  }

  fun example_foo_ref() {
    let foo = Foo { x: 3, y: false };
    let Foo { x, y } = &foo;

    // two new bindings
    //   x: &u64
    //   y: &bool
  }

  fun example_foo_ref_mut() {
    let foo = Foo { x: 3, y: false };
    let Foo { x, y } = &mut foo;

    // two new bindings
    //   x: &mut u64
    //   y: &mut bool
  }

  fun example_destroy_bar() {
    let bar = Bar { foo: Foo { x: 3, y: false } };
    let Bar { foo: Foo { x, y } } = bar;
    //             ^ nested pattern

    // two new bindings
    //   x: u64 = 3
    //   y: bool = false
  }

  fun example_destroy_baz() {
    let baz = Baz {};
    let Baz {} = baz;
  }
}
```

#### Borrowing Structs and Fields

The `&` and `&mut` operators can be used to create references to structs or fields. These examples
include some optional type annotations (e.g., `: &Foo`) to demonstrate the type of operations.

```move
module 0x2::m {
  fun example() {
    let foo = Foo { x: 3, y: true };
    let foo_ref: &Foo = &foo;
    let y: bool = foo_ref.y;  // reading a field via a reference to the struct
    let x_ref: &u64 = &foo.x;

    let x_ref_mut: &mut u64 = &mut foo.x;
    *x_ref_mut = 42;  // modifying a field via a mutable reference
  }
}
```

It is possible to borrow inner fields of nested structs:

```move
module 0x2::m {
  fun example() {
    let foo = Foo { x: 3, y: true };
    let bar = Bar { foo };

    let x_ref = &bar.foo.x;
  }
}
```

You can also borrow a field via a reference to a struct:

```move
module 0x2::m {
  fun example() {
    let foo = Foo { x: 3, y: true };
    let foo_ref = &foo;
    let x_ref = &foo_ref.x;
    // this has the same effect as let x_ref = &foo.x
  }
}
```

#### Reading and Writing Fields

If a field is copyable, you can read and copy a field's value by dereferencing the borrowed field:

```move
module 0x2::m {
  fun example() {
    let foo = Foo { x: 3, y: true };
    let bar = Bar { foo: copy foo };
    let x: u64 = *&foo.x;
    let y: bool = *&foo.y;
    let foo2: Foo = *&bar.foo;
  }
}
```

The dot operator can be used to read and copy any copyable field of a struct without explicit
borrowing and dereferencing:

```move
module 0x2::m {
  fun example() {
    let foo = Foo { x: 3, y: true };
    let x = foo.x;  // x == 3
    let y = foo.y;  // y == true

    let bar = Bar { foo };
    let foo2: Foo = *&bar.foo; // `Foo` must be copyable
    let foo3: Foo = bar.foo;   // same as the statement above
  }
}
```

Dot operators can be chained to access nested fields:

```move
module 0x2::m {
  fun example() {
    let baz = Baz { foo: Foo { x: 3, y: true } };
    let x = baz.foo.x; // x = 3;
  }
}
```

Furthermore, the dot syntax can be used to modify fields.

```move
module 0x2::m {
  fun example() {
    let foo = Foo { x: 3, y: true };
    foo.x = 42;     // foo = Foo { x: 42, y: true }
    foo.y = !foo.y; // foo = Foo { x: 42, y: false }
    let bar = Bar { foo };            // bar = Bar { foo: Foo { x: 42, y: false } }
    bar.foo.x = 52;                   // bar = Bar { foo: Foo { x: 52, y: false } }
    bar.foo = Foo { x: 62, y: true }; // bar = Bar { foo: Foo { x: 62, y: true } }
  }
}
```

The dot syntax also works via a reference to a struct:

```move
module 0x2::m {
  fun example() {
    let foo = Foo { x: 3, y: true };
    let foo_ref = &mut foo;
    foo_ref.x = foo_ref.x + 1;
  }
}
```

### Privileged Struct Operations

Most struct operations on a struct type `T` can only be performed inside the module that declares
`T`:

- Struct types can only be created ("packed"), destroyed ("unpacked") inside the module that defines
  the struct.
- The fields of a struct are only accessible inside the module that defines the struct.

Following these rules, if you want to modify your struct outside the module, you will need to
provide public APIs for them. The end of the chapter contains some examples of this.

However, struct _types_ are always visible to another module or script:

```move
// m.move
module 0x2::m {
  struct Foo has drop { x: u64 }

  public fun new_foo(): Foo {
    Foo { x: 42 }
  }
}
```

```move
// n.move
module 0x2::n {
  use 0x2::m;

  struct Wrapper has drop {
    foo: m::Foo
  }

  fun f1(foo: m::Foo) {
    let x = foo.x;
    //      ^ error! cannot access fields of `foo` here
  }

  fun f2() {
    let foo_wrapper = Wrapper { foo: m::new_foo() };
  }
}
```

By default, structs do not have visibility modifiers—all struct operations are restricted to the defining module. Move 2.4 introduces explicit visibility modifiers; see [Struct Visibility](#struct-visibility) below.

### Struct Visibility

_Since language version 2.4_

By default, struct construction, destruction, and field access are module-private, as described in [Privileged Struct Operations](#privileged-struct-operations). Move 2.4 introduces explicit visibility modifiers that allow external modules to perform these operations. See [Struct and Enum Visibility](./structs-and-enums.md) for shared rules on visibility levels, performance, restrictions, transaction arguments, and upgradability.

#### Syntax

Place the modifier before the `struct` keyword:

```move
module 0x42::shapes {
    // Accessible from any module
    public struct Point {
        x: u64,
        y: u64,
    }

    // Accessible from modules in the same package
    package struct Config {
        value: u64,
    }
}

module 0x42::lib {
    friend 0x42::consumer;

    // Accessible only from declared friend modules
    friend struct Token {
        amount: u64,
    }
}
```

#### Cross-Module Access

External modules with sufficient visibility can construct and destruct the struct, and read and mutate its fields.

```move
module 0x42::shapes {
    public struct Point {
        x: u64,
        y: u64,
    }
}

module 0x42::user {
    use 0x42::shapes::Point;

    fun mirror(p: Point): Point {
        let Point { x, y } = p;   // destruct
        Point { x: y, y: x }     // construct
    }

    fun shift_x(p: &mut Point, delta: u64) {
        p.x = p.x + delta;       // read and write to fields
    }
}
```

### Ownership

As mentioned above in [Defining Structs](#defining-structs), structs are by default linear and
ephemeral. This means they cannot be copied or dropped. This property can be very useful when
modeling real-world resources like money, as you do not want money to be duplicated or get lost in
circulation.

```move
module 0x2::m {
  struct Foo { x: u64 }

  public fun copying_resource() {
    let foo = Foo { x: 100 };
    let foo_copy = copy foo; // error! 'copy'-ing requires the 'copy' ability
    let foo_ref = &foo;
    let another_copy = *foo_ref // error! dereference requires the 'copy' ability
  }

  public fun destroying_resource1() {
    let foo = Foo { x: 100 };

    // error! when the function returns, foo still contains a value.
    // This destruction requires the 'drop' ability
  }

  public fun destroying_resource2(f: &mut Foo) {
    *f = Foo { x: 100 } // error!
                        // destroying the old value via a write requires the 'drop' ability
  }
}
```

To fix the second example (`fun destroying_resource1`), you would need to manually "unpack" the
resource:

```move
module 0x2::m {
  struct Foo { x: u64 }

  public fun destroying_resource1_fixed() {
    let foo = Foo { x: 100 };
    let Foo { x: _ } = foo;
  }
}
```

Recall that you are only able to deconstruct a resource within the module in which it is defined.
This can be leveraged to enforce certain invariants in a system, for example, conservation of money.

If on the other hand, your struct does not represent something valuable, you can add the abilities
`copy` and `drop` to get a struct value that might feel more familiar from other programming
languages:

```move
module 0x2::m {
  struct Foo has copy, drop { x: u64 }

  public fun run() {
    let foo = Foo { x: 100 };
    let foo_copy = copy foo;
    // ^ this code copies foo, whereas `let x = foo` or
    // `let x = move foo` both move foo

    let x = foo.x;            // x = 100
    let x_copy = foo_copy.x;  // x = 100

    // both foo and foo_copy are implicitly discarded when the function returns
  }
}
```

### Positional Structs

_Since language version 2.0_

A struct can be declared to have _positional fields_, fields that are not named
but numbered. Positional structs behave similarly to regular structs,
except they provide a different syntax that may be more suitable for use cases with
only a few fields.

Fields of positional structs are assigned in the order they appear. In the example
below, field `0` is of type `u64` and field `1` is of type `u8`:

```move
module 0x2::m {
  struct Pair(u64, u8);
}
```

Abilities for positional structs are declared _after_ and not before the field list,

```move
module 0x2::m {
  struct Pair(u64, u8) has copy, drop;
}
```

For pure type tags, often used for phantom types in Move code, the list of arguments
can be also completely omitted:

```move
module 0x2::m {
  struct TypeTag has copy, drop;
}
```

Values of positional structs are created and deconstructed using
`PositionalStructs(arguments)`, as shown below:

```move
module 0x2::m {
  fun work() {
    let value = Pair(1, true);
    let Pair(number, boolean) = value;
    assert!(number == 1 && boolean == true);
  }
}
```

Fields of positional structs can be accessed using the position as a field selector. For example, in the code above, `value.0` and `value.1` can be used to access the two fields without deconstructing `value`.

### Partial Patterns

_Since language version 2.0_

Patterns can use the `..` notation to match any remaining, non-listed fields in structs or variants with named fields, and omitted fields at either the beginning or end of a struct or variant with positional fields. Here are
some examples:

```move
module 0x2::m {
  struct Foo{ x: u8, y: u16, z: u32 }
  struct Bar(u8, u16, u32);

  fun foo_get_x(self: &Foo): u16 {
    let Foo{y, ..} = self;
    x
  }

  fun bar_get_0(self: &Foo): u8 {
    let Bar(x, ..) = self;
    x
  }

  fun bar_get_2(self: &Foo): u52 {
    // For positional structs, one can also put the
    // .. at the beginning.
    let Bar(.., z) = self;
    z
  }
}
```

Notice that partial patterns can currently not be used as the left-hand side of assignment.
While one can use `let Bar(x, ..) = v`, we do not yet support `let x; Bar(x, ..) = v`.

### Storing Resources in Global Storage

Structs with the `key` ability can be saved directly in
[persistent global storage](./global-storage.md). All values stored within those `key`
structs must have the `store` ability. See the [ability](./generics-and-abilities.md) and
[global storage](./global-storage.md) chapters for more detail.

### Examples

Here are two short examples of how you might use structs to represent valuable data (in the case of
`Coin`) or more classical data (in the case of `Point` and `Circle`).

#### Example 1: Coin

```move
module 0x2::m {
  // We do not want the Coin to be copied because that would be duplicating this "money",
  // so we do not give the struct the 'copy' ability.
  // Similarly, we do not want programmers to destroy coins, so we do not give the struct the
  // 'drop' ability.
  // However, we *want* users of the modules to be able to store this coin in persistent global
  // storage, so we grant the struct the 'store' ability. This struct will only be inside of
  // other resources inside of global storage, so we do not give the struct the 'key' ability.
  struct Coin has store {
    value: u64,
  }

  public fun mint(value: u64): Coin {
    // You would want to gate this function with some form of access control to prevent
    // anyone using this module from minting an infinite amount of coins.
    Coin { value }
  }

  public fun withdraw(coin: &mut Coin, amount: u64): Coin {
    assert!(coin.value >= amount, 1000);
    coin.value = coin.value - amount;
    Coin { value: amount }
  }

  public fun deposit(coin: &mut Coin, other: Coin) {
    let Coin { value } = other;
    coin.value = coin.value + value;
  }

  public fun split(coin: Coin, amount: u64): (Coin, Coin) {
    let other = withdraw(&mut coin, amount);
    (coin, other)
  }

  public fun merge(coin1: Coin, coin2: Coin): Coin {
    deposit(&mut coin1, coin2);
    coin1
  }

  public fun destroy_zero(coin: Coin) {
    let Coin { value } = coin;
    assert!(value == 0, 1001);
  }
}
```

#### Example 2: Geometry

```move
module 0x2::point {
  struct Point has copy, drop, store {
    x: u64,
    y: u64,
  }

  public fun new(x: u64, y: u64): Point {
    Point {
      x, y
    }
  }

  public fun x(p: &Point): u64 {
    p.x
  }

  public fun y(p: &Point): u64 {
    p.y
  }

  fun abs_sub(a: u64, b: u64): u64 {
    if (a < b) {
      b - a
    }
    else {
      a - b
    }
  }

  public fun dist_squared(p1: &Point, p2: &Point): u64 {
    let dx = abs_sub(p1.x, p2.x);
    let dy = abs_sub(p1.y, p2.y);
    dx*dx + dy*dy
  }
}
```

```move
module 0x2::circle {
  use 0x2::point::{Self, Point};

  struct Circle has copy, drop, store {
    center: Point,
    radius: u64,
  }

  public fun new(center: Point, radius: u64): Circle {
    Circle { center, radius }
  }

  public fun overlaps(c1: &Circle, c2: &Circle): bool {
    let dist_squared_value = point::dist_squared(&c1.center, &c2.center);
    let r1 = c1.radius;
    let r2 = c2.radius;
    dist_squared_value <= r1*r1 + 2*r1*r2 + r2*r2
  }
}
```

## Enums

_Since language version 2.0_

Enum types are similar to struct types but support defining multiple _variants_ of the data layout. Each variant has its own distinct set of fields. Enum variants are supported in expressions, with tools for testing, matching, and deconstructing them.

### Declaration of Enum Types

An enum type declaration lists the number of different variants, as seen in the example below:

```move
enum Shape {
    Circle{radius: u64},
    Rectangle{width: u64, height: u64}
}
```

There can be zero or more fields for an enum variant. If no arguments are given, the braces can also be omitted, declaring simple values:

```move
enum Color {
  Red, Blue, Green
}
```

Like struct types, enum types can have abilities. For example, the `Color` enum type would be appropriately declared as copyable, droppable, and storable, like primitive number types:

```move
enum Color has copy, drop, store, key { Red, Blue, Green }
```

Enum types can also have the `key` ability and appear as roots of data in global storage. A common usage of enums in this context is versioning of data:

```move
enum VersionedData has key {
  V1{name: String},
  V2{name: String, age: u64},
}
```

Similar to structs, enum types can be generic and take positional arguments. For example, the type below represents a generic result type, where the variant constructors use positional instead of named arguments (see also [positional structs](./structs-and-enums.md#positional-structs)).

```move
enum Result<T> has copy, drop, store {
  Err(u64),
  Ok(T)
}
```

### Constructing Enum Values

An enum value is constructed similarly to a struct value:

```move
let s: String;
let data = VersionedData::V1{name: s};
```

If the enum variant has no fields, the braces can also be omitted:

```move
let color = Color::Blue;
```

### Name Resolution for Enum Variants

The variant names for an enum need to be qualified by the enum type name, as in `VersionedData::V1`.

> Note: Aliasing via the `use` clause is currently not supported for enum variants, but will be added in later language versions.

In certain cases (such as match expressions, below), the Move compiler can infer the enum type from the context, and the qualification by the type name may be omitted:

```move
fun f(data: VersionedData) {
  match (data) { V1{..} => .., ..} // simple variant name OK
}
```

### Matching Enum Values

The value of an enum value can be inspected using a match expression. For example:

```move
fun area(self: &Shape): u64 {
    match (self) {
        Circle{radius}           => mul_with_pi(*radius * *radius),
        Rectangle{width, height} => *width * *height
    }
}
```

Notice above that the value matched is an immutable reference to an enum value. A match expression can also consume a value, or match over a mutable reference for interior updates:

```move
fun scale_radius(self: &mut Shape, factor:  u64) {
    match (self) {
        Circle{radius: r} => *r = *r * factor,
        _                 => {} // do nothing if not a Circle
  }
}
```

The patterns provided in the match expression are evaluated sequentially, in order of textual occurrence, until a match is found. It is a compile-time error if not all known patterns are covered.

Patterns can be nested and contain conditions, as in the following example:

```move
let r : Result<Result<u64>> = Ok(Err(42));
let v = match (r) {
  Ok(Err(c)) if c < 42  => 0,
  Ok(Err(c)) if c >= 42 => 1,
  Ok(_)                 => 2,
  _                     => 3
};
assert!(v == 1);
```

Notice that in the above example, the last match clause (`_`) covers both patterns `Ok(Err(_))` and `Err(_)`. Although at execution time, the earlier clauses match `Ok(Err(c))` for all values of `c`, the compiler cannot be sure all cases are covered due to the conditionals: conditions in match expressions are not considered when tracking coverage. Thus the first two clauses in the match expression above are not sufficient for match completeness, and an additional clause is required to avoid a compiler error.

### Testing Enum Variants

With the `is` operator, one can examine whether a given enum value is of a given variant:

```move
let data: VersionedData;
if (data is VersionedData::V1) { .. }
```

The operator allows specifying a list of variants, separated by "`|`" characters. The variants need not be qualified by the enum name if the type of the expression being tested is known:

```move
assert!(data is V1|V2);
```

### Selecting From Enum Values

It is possible to directly select a field from an enum value. Recall the definition of versioned data:

```move
enum VersionedData has key {
  V1{name: String},
  V2{name: String, age: u64},
}
```

One can write code as below to directly select the fields of variants:

```move
let s: String;
let data1 = VersionedData::V1{name: s};
let data2 = VersionedData::V2{name: s, age: 20};
assert!(data1.name == data2.name);
assert!(data2.age == 20);
```

Notice that field selection aborts if the enum value has no variant with the given field. This is the case for `data1.age`.
The abort code used for this case is `0xCA26CBD9BE0B0001`. In terms of the `std::error` convention, this code has
category `std::error::INTERNAL` and reason `1`.

Field selection is only possible if the field is uniquely named and typed throughout all variants. Thus, the following yields a compile-time error:

```move
enum VersionedData has key {
  V1{name: String},
  V2{name: u64},
}

data.name
 // ^^^^^ compile time error that `name` field selection is ambiguous
```

### Using Enums Patterns in Lets

An enum variant pattern may be used in a `let` statement:

```move
let data: VersionData;
let V1{name} = data;
```

Unpacking the enum value will abort if the variant is not the expected one. To ensure that all variants of an enum are handled, a `match` expression is recommended instead of a `let`. The `match` is checked at compile time, ensuring that all variants are covered. In some cases, tools like the Move Prover can be used to verify that unexpected aborts cannot happen with a `let`.

### Destroying Enums via Pattern Matching

Similar to struct values, enum values can be destroyed by explicitly unpacking them. Enums can be unpacked with pattern matching in a `match` expression, enum pattern in a `let` binding, or enum pattern in an assignment.

```move
// Note: `Shape` has no `drop` ability, so must be destroyed with explicit unpacking.
enum Shape {
    Circle{radius: u64},
    Rectangle{width: u64, height: u64}
}

fun destroy_empty(self: Shape) {
    match (self) {
        Shape::Circle{radius} => assert!(radius == 0),
        Shape::Rectangle{width, height: _} => assert!(width == 0),
    }
}

fun example_destroy_shapes() {
    let c = Shape::Circle{radius: 0};
    let r = Shape::Rectangle{width: 0, height: 0};
    c.destroy_empty();
    r.destroy_empty();
}
```

### Enum Visibility

_Since language version 2.4_

By default, enum construction, deconstruction, matching, and field selection are restricted to the defining module. Move 2.4 introduces explicit visibility modifiers for enums, following the same model described in [Struct and Enum Visibility](./structs-and-enums.md).

#### Syntax

Place the modifier before the `enum` keyword. For enums with abilities, the `has` clause follows the variant list as usual:

```move
module 0x42::types {
    // Accessible from any module
    public enum Color has copy, drop {
        Red, Green, Blue,
    }

    // Accessible from modules in the same package
    package enum Status has drop {
        Active,
        Inactive,
    }
}

module 0x42::lib {
    friend 0x42::consumer;

    // Accessible only from declared friend modules
    friend enum Event has drop {
        Created, Updated, Deleted,
    }
}
```

#### Cross-Module Access

External modules with sufficient visibility can construct and destruct the enum, test variants, select fields from variants, and mutate them.

```move
module 0x42::types {
    public enum Shape has drop {
        Circle { radius: u64 },
        Rectangle { width: u64, height: u64 },
    }
}

module 0x42::user {
    use 0x42::types::Shape;

    fun area(s: Shape): u64 {
        match (s) {
            Shape::Circle { radius } => radius * radius,
            Shape::Rectangle { width, height } => width * height,
        }
    }

    fun is_circle(s: &Shape): bool {
        s is Shape::Circle
    }
}
```

### Enum Type Upgrade Compatibility

An enum type can be upgraded by another enum type if the new type only adds new variants at the end of the variant list. All variants present in the old enum type must also appear in the new type, in the same order and starting from the beginning. Consider the `VersionedData` type, which might have begun with a single version:

```move
enum VersionedData has key {
  V1{name: String}
}
```

This type could be upgraded to the version we used so far in this text:

```move
enum VersionedData has key {
  V1{name: String},
  V2{name: String, age: u64},
}
```

The following upgrade would not be allowed, since the order of variants must be preserved:

```move
enum VersionedData has key {
  V2{name: String, age: u64},   // not a compatible upgrade
  V1{name: String},
}
```

## Struct and Enum Visibility

_Since language version 2.4_

By default, struct and enum construction, destruction, and field access are restricted to the defining module. Move 2.4 introduces explicit visibility modifiers that allow external modules to perform these operations.

### Visibility Levels

Three modifiers are available, mirroring the function visibility keywords:

| Modifier  | Accessible from                                     |
| --------- | --------------------------------------------------- |
| `public`  | Any module                                          |
| `package` | All modules in the same package                     |
| `friend`  | Modules declared as `friend` in the defining module |

`public(package)` and `public(friend)` are accepted as aliases for `package` and `friend` respectively, but are discouraged and will be deprecated. Prefer the shorthand forms.

### Performance Consideration

Cross-module type operations are currently compiled into function calls rather than direct bytecode instructions. Thus, they are not a zero-cost abstraction yet: they are more expensive than the equivalent operations performed within the defining module. This is expected to change in the future with VM improvements. In the meantime, use visibility modifiers only when the cross-module access is genuinely needed.

### Restrictions

**Types with the `key` ability cannot have visibility modifiers.** Types stored as roots in global storage must remain module-private to preserve global storage access control.

```move
// ERROR: types with the key ability cannot be public, package, or friend
public struct Resource has key { value: u64 }
public enum VersionedData has key { V1 { name: vector<u8> } }
```

**Global storage operations remain module-only.** Even for `public` types, `move_to`, `move_from`, `borrow_global`, and `borrow_global_mut` are restricted to the defining module.

### Transaction Arguments

`public` structs and enums can be passed as entry and view function arguments if the type has the `copy` ability and does not have the `key` ability. All field types must themselves be valid argument types, recursively.

```move
module 0x42::types {
    public struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    public enum Direction has copy, drop {
        North, South, East, West,
    }

    // Both Point and Direction can be passed directly as transaction arguments
    entry fun move_to_point(s: &signer, destination: Point) { .. }
    entry fun move_player(s: &signer, dir: Direction) { .. }
}
```

### Upgradability

In a package upgrade, struct and enum visibility may change in the
following ways. The general principle is that no transition is allowed
which would break code outside the defining package: `public` can never
be narrowed, but transitions among the package-internal levels are safe
because their consumers live inside the same package and are republished
atomically with the upgrade.

- A private type can be upgraded to `package`, `friend`, or `public`.
- A `friend` or `package` type can be upgraded to `public`.
- A `friend` or `package` type can be narrowed back to private.
