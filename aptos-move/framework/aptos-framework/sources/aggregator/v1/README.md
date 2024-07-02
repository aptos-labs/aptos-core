# Design Rationale

## Aggregators

Aggregator is a parallelizable integer that supports addition and subtraction.
Unlike integer, aggregator has a user-defined `limit` which specifies when
the value of aggregator overflows. Similarly to unsigned integers, the value
of an aggregator underflows when going below zero.

Both additions and subtractions are executed speculatively, and thus can be
easily parallelized. On underflow or overflow, these operations are guaranteed
to abort. Using these operations is encouraged.

Reading an aggregator's value "materializes" the speculative value, and it
can involve reading from the storage or checking for underflow or overflow.
In general, using this operation is discouraged, or at least it should be used
as rarely as possible.

## Aggregator factory

Unfortunately, aggregators cannot be part of a resource. At the moment, Move
does not allow fine-grained access to resource fields, which ruins performance
benefits aggregators can provide. In addition, getting the value of the field of
a resource from storage is not possible without hardcoding the struct layout.
For example, given a struct

```move
struct Foo<A> has key {
    a: A,
    b: u128,
}
```

there is no clean way of getting the value of `Foo::a` without knowing that the
offset is 0.

To mitigate the problem, we store aggregators as table items. Recall that every
item stored in the table is uniquely identified by `(handle, key)` pair: `handle` 
identifies a table instance, and `key` identifies an item within the table. Now,
if aggregator is a table item, it can be easily queried from storage and has a
fine-grained access.

To create an aggregator, one can use an `AggregatorFactory`. It is a resource
which contains a single `phantom_table` field. When the factory is initialized,
this field is used to generate a unique table `handle` which is passed to all
new aggregators. When a new aggregator instance is created, it has a unique
`key` which together with the `handle` is stored in `Aggregator` struct.
