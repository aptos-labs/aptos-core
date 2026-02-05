# Mono Runtime

Mono runtime is a new style of Move VM execution engine based on _monomorphization_. This here is a proof-of-concept prototype.

Some general design principles:

- Code is assumed to be monomorphized and compiled from the Move binary format into the new `MonoCode` representation _at execution time_.
- Data is represented in linear contiguous memory and in its native representation. A `u8` is represented as a byte, `u16` as two bytes, and so on. For structs, fields are packed side-by-side; nested structs are inlined in the outer struct. Vectors are currently not implemented, but the idea is manage them on the heap, with fixed size in their container. Hence, every type has a fixed size representation.
- The code and the executor are inspired by the CLR (.Net runtime): instructions are generic except the _size_ of the data they are working on. For example, reading or writing a reference is 'sized' w.r.t. to the type behind the reference. This allows a uniform handling of different kind of primitive types and structs with a small, yet high-performing instruction set.
- The memory of the executor is partitioned into a traditional contiguous stack and a heap. On the stack, call frames are similar represented like structs, that is, parameters and locals are packed side-by-side according to their size. 
- The implementation uses unsafe code to manage the contiguous memory representation, namely pointer casting. In the contiguous memory model, a slice of a byte region can have a mutable reference whereas another disjoint region can have an immutable reference. This is not expressible in safe Rust. However, for safe Move code, safety should be maintained. _Any unsafe code does not affect integrity of the Rust process_, as we never reinterpret Rust data structures, but only Move structures. 
