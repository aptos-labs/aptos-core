# Overview

Rust's borrow checker rejects some programs that are safe. Two of its
limitations matter here.

## It does not understand partial borrows at function boundaries

```rust
struct Foo {
    x: String,
    y: u64,
}

impl Foo {
    fn x(&self) -> &str {
        &self.x
    }

    fn inc_y(&mut self) {
        self.y += 1;
    }
}

fn do_something(foo: &mut Foo) {
    let x = foo.x();
    foo.inc_y(); // error! all of `foo` is held by the reference `x`
    println!("{x}");
}
```

Here `fn x` and `fn inc_y` touch entirely disjoint fields, so conceptually it is safe to interleave them in arbitrary order.

However, the compiler checks `do_something` against the two signatures only, never the bodies. `fn x(&self) -> &str` says that the output borrows from `self`, with no way to say which part, so the compiler has to make the conservative assumption that it borrows all of it. That conflicts with `inc_y`, which wants exclusive control over the same `self`.

The limitation is specific to function boundaries. With the two bodies written inline, the compiler tracks each field separately and accepts the same code:

```rust
fn do_something(foo: &mut Foo) {
    let x = &foo.x;
    foo.y += 1; // fine
    println!("{x}");
}
```

A hypothetical language extension that lets a signature name the fields it borrows would solve this problem:

```rust
fn x(&self.x) -> &str;      // borrows only `self.x`

fn inc_y(&mut self.y);      // borrows only `self.y`
```

Rust has no such feature today, so people work around the limitation. Two workarounds are common.

### Macros

Since the inlined code is accepted, replace the methods with macros, which expand at the call site and so never create a function boundary:

```rust
macro_rules! x {
    ($foo:expr) => {
        $foo.x.as_str()
    };
}

macro_rules! inc_y {
    ($foo:expr) => {
        $foo.y += 1
    };
}

fn do_something(foo: &mut Foo) {
    let x = x!(foo);
    inc_y!(foo); // fine: the compiler sees `foo.x` and `foo.y`
    println!("{x}");
}
```

This keeps the borrow checker fully in charge, at the cost of ergonomics.

### Cells

Keep the methods, but make the mutating one take `&self` as well, and move the field it changes into a cell:

```rust
struct Foo {
    x: String,
    y: UnsafeCell<u64>,
}

impl Foo {
    fn x(&self) -> &str {
        &self.x
    }

    fn inc_y(&self) {
        // SAFETY: nothing else is accessing `y` during this call.
        let y = unsafe { &mut *self.y.get() };
        *y += 1;
    }
}

fn do_something(foo: &Foo) {
    let x = foo.x();
    foo.inc_y(); // fine: both calls borrow `foo` shared
    println!("{x}");
}
```

In this document, to *borrow a cell* means to obtain a reference to the value it holds, as `&mut *self.y.get()` does here. It does not mean `&self.y`, a reference to the `UnsafeCell` itself, which is harmless.

> Side note: Rust technically calls this a *reborrow*, because the reference is made by going through an existing pointer rather than by borrowing a variable directly. This document says *borrow* throughout.

This effectively tells the compiler: don't check these borrows, I will handle them. Correctness then rests on the implementation being right (`UnsafeCell`) or on a check at run time (`RefCell`).

## It does not understand "taking turns"

Conceptually, having two or more references to the same data can still be safe, as long as their uses do not overlap: each use finishes, and leaves nothing behind that depends on the data, before the next one begins.

```rust
struct Foo {
    x: u64,
}

impl Foo {
    fn bump(&mut self) {
        self.x += 1;
    }
}

fn do_something() {
    let mut foo = Foo { x: 0 };

    let r1 = &mut foo;
    let r2 = &mut foo; // error! `foo` is still held by `r1`

    r1.bump();
    r2.bump();
    r1.bump();
    r2.bump();
}
```

Here each `bump` runs to completion before the next one starts, and keeps nothing once it returns, so the calls never interfere, whichever reference they go through.

However, the compiler does not reason about when the data is actually accessed. It reasons about how long each reference stays in use: `r1` is used again after `r2` is created, so both are in use over the same stretch of code, and two `&mut` to the same data are never allowed to be.

In this toy example, `r1` and `r2` look pointless. In real code they can be two long-lived holders of the same object, for example two handles that each keep a reference to the pool they came from.

### Cells

Again, cells can be used to work around this:

```rust
struct Foo {
    x: UnsafeCell<u64>,
}

impl Foo {
    fn bump(&self) {
        // SAFETY: nothing else is accessing `x` during this call.
        let x = unsafe { &mut *self.x.get() };
        *x += 1;
    }
}

fn do_something() {
    let foo = Foo { x: UnsafeCell::new(0) };

    let r1 = &foo;
    let r2 = &foo; // fine: shared references may coexist

    r1.bump();
    r2.bump();
    r1.bump();
    r2.bump();
}
```

This is "taking turns" written in a form the compiler accepts. The exclusive reference is created inside `bump` and gone when it returns, so exclusivity lasts only as long as each access, not as long as `r1` and `r2` exist.

The compiler no longer checks any of this, so the code must honor the "taking turns" contract by hand: **at any moment, the live references obtained from the cell are either one exclusive reference or any number of shared ones.** This is the ordinary borrow rule. Only who upholds it has changed.

The cell is private, so only the type's own code can name it. Whenever that code borrows the cell, it must make sure of two things:

1. **Nothing outlives the call.** The borrow, and anything derived from it, such as a reference into the data, is gone when the function returns. It is not returned, not stored, and not kept by anything it was passed to.
2. **Conflicting borrows never overlap.** Two borrows of the cell conflict unless both are shared. While a borrow is live, from the point it is made to its last use, nothing makes a conflicting one: not the same function, not another method of the same object, and not outside code, such as a callback or a trait object, that could call back into it.

`bump` honors both trivially: it returns nothing and calls nothing. The contract gets harder to keep as methods grow, call each other, and accept callbacks.

# How contexts run into both

Inside the VM, a *context* is a common construct: one object that bundles the components a piece of code needs and exposes them through methods. 

The native context is an example. A native function gets a single `ctx`, and through it reaches the heap, global storage, the gas meter, and so on.

Two things about a context make it collide with the borrow checker:

- It is a single object reached through methods, so every access crosses a function boundary. Macros would avoid the boundary, but only by exposing the context's fields to every caller.
- Callers obtain references or handles to parts of the context, and often need several at once, or need to call the context again while holding one.

A minimal context with both properties, mimicking Mono Move's `NativeContext`:

```rust
struct Context<'a> {
    gas: u64,
    heap: &'a mut Heap,
    pool: Pool,
}

struct Handle<'a> {
    pool: &'a Pool,
    slot: usize,
}

impl Context<'_> {
    fn charge_gas(&mut self, amount: u64) {
        self.gas -= amount;
    }

    /// Allocates an object and returns a handle to it.
    fn alloc(&mut self, value: u64) -> Handle<'_> {
        self.heap.objects.push(value);                      // mutates the heap
        self.pool.roots.push(self.heap.objects.len() - 1);  // mutates the pool
        Handle { pool: &self.pool, slot: self.pool.roots.len() - 1 }
    }
}

fn native(ctx: &mut Context) {
    let a = ctx.alloc(1);
    ctx.charge_gas(10);    // error! `ctx` is still held by `a`
    let b = ctx.alloc(2);  // error! `ctx` is still held by `a`
    ...
}
```

Allocating, charging gas and allocating again is about the most ordinary thing a native does, and both calls are rejected, each for one of the two reasons:

- **`charge_gas` touches only `gas`, while `a` refers only to the pool.** The two are disjoint fields, but the signatures say that `a` borrows all of `ctx` and that `charge_gas` needs all of it. This is the partial-borrow limitation.
- **`alloc` must add a slot to the pool, which `a` does refer to.** But `a` is not reading the pool at that moment; it will only look up its slot later. This is the "taking turns" limitation. (`alloc` also mutates the heap, which is the first limitation again.)

Cells resolve both:

```rust
struct Context<'a> {
    gas: UnsafeCell<u64>,            // for partial borrows
    heap: UnsafeCell<&'a mut Heap>,  // for partial borrows
    pool: Pool,
}

struct Pool {
    roots: UnsafeCell<Vec<usize>>,   // for "taking turns"
}

impl Pool {
    fn root(&self, object: usize) -> Handle<'_> {
        // SAFETY: nothing else is accessing `roots` during this call.
        let roots = unsafe { &mut *self.roots.get() };
        roots.push(object);
        Handle { pool: self, slot: roots.len() - 1 }
    }
}

impl Context<'_> {
    fn charge_gas(&self, amount: u64) {
        // SAFETY: nothing else is accessing `gas` during this call.
        let gas = unsafe { &mut *self.gas.get() };
        *gas -= amount;
    }

    fn alloc(&self, value: u64) -> Handle<'_> {
        // SAFETY: nothing else is accessing `heap` during this call.
        let heap = unsafe { &mut **self.heap.get() };
        heap.objects.push(value);
        self.pool.root(heap.objects.len() - 1)
    }
}

fn native(ctx: &Context) {
    let a = ctx.alloc(1);
    ctx.charge_gas(10);    // fine
    let b = ctx.alloc(2);  // fine
    ...
}
```

What callers get from this design:

- **A natural API.** A caller can allocate again, or charge gas, while holding a value.
- **A safe API.** A caller may make any calls, in any order, while holding any number of handles, without having to worry about memory corruption: no sequence of calls can cause undefined behavior.

The cost falls entirely on the implementation.

# The contract on the implementation

By putting a value into a cell, you change the **granularity and duration** of its borrows, and you take over their checking from the compiler:

- **Granularity: per cell, not per containing struct.** This naturally addresses the partial-borrow limitation, and comes with no additional contract for the implementation to follow.
- **Duration: per access, not per reference.** This addresses the "taking turns" limitation, but only if the implementation follows a contract: at any moment, the live references obtained from a given cell are either one exclusive reference or any number of shared ones.
  - This holds for every cell, including one that exists only because of partial borrows, such as `gas`.

For a context, this means two things:

- **Nothing outlives the call.** A borrow of a cell, or anything derived from it, is not returned, not stored, and not kept by anything it was passed to. A method hands out handles, never a reference into what a cell holds.
- **Conflicting borrows never overlap.** Two borrows of the same cell conflict unless both are shared. While a borrow is live, from the point it is made to its last use, nothing makes a conflicting one: not the same function, not another method of the context, and not code that was handed the context or supplied by the caller.
  - Specifically, a function may borrow the same cell several times, or call a method that does, as long as the borrows take turns.

The cells are private, so only the context's own code can make a borrow. That keeps the reasoning almost entirely local: code that merely calls the context cannot break the contract, and needs no checking.

The one exception is code that the context itself runs while a borrow is live, such as a callback, or a trait method of a value supplied by the caller. The context must either not run such code at that point, or know that it cannot call back into the context.

The hazard is that, as the code evolves, it is not obvious when a change breaks one of the rules: a call added inside a helper can make a method several levels up re-enter a cell it is holding, without that method being touched at all.

So rather than rely on every change being reasoned through by hand, we would like the rules checked automatically.

# Automating the check

## Why the check has to be conservative

Checking the two constraints precisely would mean redoing the borrow checker's work at the granularity of a cell, and then going beyond it: since every method takes `&self`, a signature no longer says what a callee touches, so the check would also have to follow calls across functions, trait objects and crates.

That is a lot of machinery, so we propose something simpler instead: a small set of rules that are stricter than necessary, but still easy to implement, explain and follow. The trade-off is that they will occasionally reject code that is actually fine.

In practice this costs little. The rules do not limit what can be implemented, only how it is written, and one simple pattern satisfies them: borrow the cells a method needs once, at the top of the method, and pass the references down to helper functions as ordinary parameters. From there on the compiler checks every use, as it would for any other reference. The native context was already written almost entirely this way, and bringing it in line with the rules took two small changes.

The two constraints get different treatment. The first, "nothing outlives the call", can be handed back to the compiler and checked exactly. The second, "conflicting borrows never overlap", is where the conservative rules come in.

## Nothing outlives the call: let the compiler check it

The compiler normally stops a reference from leaving a function it should not leave. It does not stop a borrow of an `UnsafeCell`. All an `UnsafeCell` gives out is a raw pointer, through `get()`, and a reference made from a raw pointer carries no lifetime that ties it to the function:

```rust
fn first(&self) -> &u64 {
    let heap = unsafe { &mut **self.heap.get() };
    &heap.objects[0]  // compiles: a reference into the cell has escaped
}
```

We can give it such a lifetime. Tell the compiler to treat the borrow as if it borrowed from a local variable, an *anchor*, and the compiler's ordinary rule takes over: a reference to a local cannot leave the function.

```rust
/// Shortens a borrow to the lifetime of `anchor`. Does nothing at run time.
fn bound<'s, T: ?Sized>(_anchor: &'s mut (), r: &'s mut T) -> &'s mut T {
    r
}

fn first(&self) -> &u64 {
    let mut anchor = ();
    let heap = bound(&mut anchor, unsafe { &mut **self.heap.get() });
    &heap.objects[0]  // error! cannot return value referencing local variable `anchor`
}
```

The same error stops a borrow, or anything derived from it, from being returned, stored through a parameter, or kept by a function it is passed to. Nothing is left for a separate check to analyze.

The anchor is borrowed exclusively, which buys one more thing: two borrows made from the same anchor cannot be live together, so a borrow inside a loop cannot pile up across iterations either.

What remains to check is only that every borrow is anchored, and that the anchor really is a local of the same function. An anchor that is a parameter or a field would outlive the function and bound nothing. In practice the two lines would sit behind a small macro, so that they cannot be written wrong.

The examples below leave the anchor out for brevity.

## Conflicting borrows never overlap: conservative rules

We propose replacing this constraint with stricter ones that are much easier to check, through three simplifications:

- **Treat a borrow as live for the whole function**, not from the point it is made to its last use. A function then borrows a cell at most once, and "while a borrow is live" becomes "anywhere in a function that borrows".

  ```rust
  fn f(&self) {
      {
          let heap = unsafe { &mut **self.heap.get() };
          heap.objects.push(1);
      }               // the borrow is finished here...
      self.alloc(2);  // ...but this is still rejected: `alloc` borrows `heap` too
  }
  ```

  Siblings are still allowed, however. "Taking turns" is a property that composes: calling functions that each take turns, in whatever order, still takes turns, as long as the calling function does not borrow the cell itself.

  ```rust
  fn f(&self) {
      self.alloc(1);  // `alloc` takes turns on `heap`
      self.alloc(2);  // fine: so does this call, and `f` itself borrows nothing
  }
  ```

- **Treat shared and exclusive borrows alike.** Two shared borrows of a cell are rejected like any other pair.

  ```rust
  fn len(&self) -> usize {
      let heap = unsafe { &**self.heap.get() };  // shared
      heap.objects.len()
  }

  fn g(&self) -> usize {
      let heap = unsafe { &**self.heap.get() };  // shared
      self.len()                                 // rejected, although two `&Heap` may coexist
  }
  ```

- **Assume worst-case behavior for opaque code.** Code the check cannot look inside, such as another crate, a trait object, a generic parameter, a callback or a destructor, is assumed to borrow every cell if it has any way to reach the struct. The check does not recognise every case of opaque code; see [Limitations](#limitations).

  ```rust
  fn h(&self, on_alloc: impl Fn()) {
      let heap = unsafe { &mut **self.heap.get() };
      on_alloc();  // rejected: the callback might call back into `self`,
                   // even if no caller ever passes one that does
  }
  ```

## Proposed scope

The check would be opt-in: it applies only to structs that are explicitly listed. A struct is opted in by adding its full path to the lint's configuration in `dylint.toml`, at the repository root:

```toml
[aptos_lints.unsafe_cell_reborrow]
structs = [
    "mono_move_runtime::native_context::ProductionNativeContext",
    "mono_move_core::root_pool::RootPool",
]
```

For a listed struct, every `UnsafeCell` field is checked. These fields are the *cells* that the rules below refer to. An `UnsafeCell` in any other struct is ignored.

If an entry in the list does not match a struct, the check reports an error. Otherwise, renaming a struct would silently switch the check off for it. The check runs on one crate at a time, so it cannot see an entry whose crate does not exist or is not one it runs on; a test in the lint crate, `listed_structs_are_in_linted_crates`, reports those.

The check examines every function and closure of the crate that defines a listed struct, since cell fields are private and that is the only code able to name one.

Although the native context is what motivated the check, nothing in it is specific to contexts. The `takes_turns` property defined below is, in effect, a checked relaxation of Rust's borrow rule: it accepts a pattern the borrow checker rejects, long-lived shared references with brief exclusive access behind them, and still checks it statically, within the [limitations](#limitations) listed below. Any other struct that runs into the same two limitations can use the pattern and be enrolled in the same way.

## Proposed rules

Code is *opaque* if the check cannot look inside it: another crate, a trait object, a generic parameter, a callback.

For every function `f` of the crate, the check computes two sets of cells:

```text
held(f)    = the cells borrowed directly in f's own body

borrows(f) = held(f)                                         // what a call to f may borrow
           ∪ borrows(g)   for every function g of this crate that f calls
           ∪ all cells    if f passes the struct to opaque code
           ∪ all cells    if f owns a value whose destructor can reach the struct
```

`borrows` is recursive: it follows calls through the whole crate, including the ones implied by operators, `Deref` and closures.

A function passes the check if it *takes turns*:

```text
takes_turns(f) =
    // 1. borrowing
    f touches a cell only by borrowing it, in the anchored form
    f borrows each cell at most once

    // 2. no re-entry through code the check can see
    for every function g of this crate that f calls:
        held(f) ∩ borrows(g) = ∅

    // 3. no re-entry through opaque code
    if held(f) ≠ ∅:
        f passes nothing that can reach the struct to opaque code
        f calls nothing on a callback, trait object or generic value
        f takes no generic parameter
        f owns no value whose destructor can reach the struct
```

The crate passes if `takes_turns(f)` holds for every function, plus one condition that is not about a single function:

- **The struct is never hidden from the check.** A value that can reach the struct is not put behind a generic parameter, a trait object or a closure argument, where the check could no longer tell what is called on it. Nor is it cast with `as` to a type that no longer shows the struct, such as `*const ()`.

Part 3 has one exception: the struct's own fields. A function may call a method on a field of the struct, or pass a field to other code, while it holds a cell, even if the field is a trait object. The check assumes, without verifying it, that a field was built before the struct and so has no way to call back into it.

Siblings need no rule of their own. A function that borrows nothing itself has an empty `held`, so parts 2 and 3 ask nothing of it, however many functions it calls.

The compiler enforces the first constraint, "nothing outlives the call", through the anchor that part 1 requires. Everything else enforces the second, "conflicting borrows never overlap".

# Limitations

The check, as currently implemented, is not a proof. It is stricter than necessary in most places, but it has known gaps where it accepts code that could borrow a cell twice. Passing it rules out the common mistakes; it does not guarantee that a listed struct is free of undefined behaviour. The `unsafe` blocks still need review.

The check decides whether opaque code can reach the struct from the types of what that code is handed in a call. The known gaps are the two ways in which that falls short.

## A pointer to the struct that was stored earlier

The check does not track such a pointer. A trait object that kept one can therefore be called while a cell is held, without the check seeing that it reaches the struct.

This takes two forms, which differ in where the trait object comes from:

- **A trait object from outside the struct.** A function that holds a cell may not take a trait object as a parameter, even inside another crate's type such as `fmt::Formatter`, and may not call a method on one. But it may obtain one some other way, such as from a call's return value, and pass it to opaque code, which can then call it.

  ```rust
  // In another crate: a registry that keeps one sink as a trait object.
  thread_local! {
      static SINK: RefCell<Option<Box<dyn fmt::Write>>> = RefCell::new(None);
  }

  pub fn set_sink(sink: impl fmt::Write + 'static) {
      SINK.with(|s| *s.borrow_mut() = Some(Box::new(sink)));
  }

  pub fn take_sink() -> Box<dyn fmt::Write> {
      SINK.with(|s| s.borrow_mut().take().unwrap())
  }
  ```

  ```rust
  // In this crate.
  struct Sink { ctx: *const Ctx }

  impl fmt::Write for Sink {
      fn write_str(&mut self, _: &str) -> fmt::Result {
          unsafe { (*self.ctx).alloc(1) };  // borrows `heap`
          Ok(())
      }
  }

  impl Ctx {
      fn alloc(&self, v: u64) {
          let heap = unsafe { &mut **self.heap.get() };
          heap.objects.push(v);
      }

      fn register(&self) {
          // accepted: `register` borrows nothing itself. The sink becomes a
          // trait object inside the other crate, where the check cannot see it.
          other_crate::set_sink(Sink { ctx: self });
      }

      fn f(&self) {
          let heap = unsafe { &mut **self.heap.get() };

          // accepted: the call is not handed the struct
          let mut sink: Box<dyn fmt::Write> = other_crate::take_sink();

          // accepted: `fmt::write` is not handed the struct, as far as the
          // check can tell, but it calls `sink.write_str`, which calls `alloc`
          let _ = fmt::write(&mut *sink, format_args!("x"));
      }
  }
  ```

  Rejecting this was tried and is not practical: the VM's error type contains a trait object, so every `?` in a function that holds a cell would be rejected.

- **A trait object that is a field of the struct.** Calls on the struct's own fields are exempt from part 3. The check assumes, without verifying it, that a field was built before the struct and was never given a pointer to it afterwards.

  ```rust
  struct Ctx<'a> {
      heap: UnsafeCell<&'a mut Heap>,
      hook: &'a dyn Hook,
  }

  impl Ctx<'_> {
      fn register(&self) {
          self.hook.attach(self);  // accepted: `register` borrows nothing itself,
                                   // but the hook may keep a pointer to the context
      }

      fn alloc(&self, v: u64) {
          let heap = unsafe { &mut **self.heap.get() };
          self.hook.on_alloc();    // accepted: `hook` is a field, but it can call
                                   // `alloc` again through the pointer it kept
          heap.objects.push(v);
      }
  }
  ```

## A pointer to the struct whose type was erased

The check rejects the ways in which code of this crate can erase the type: turning the value into a trait object, passing it to a generic parameter, and an `as` cast. But a function that holds no cell may hand the struct to code outside the crate, and that code can return it under another type. The pointer can then be passed to opaque code while a cell is held, without the check seeing that it reaches the struct.

```rust
impl Ctx {
    fn erased(&self) -> *const () {
        // accepted: `erased` borrows nothing itself, and `cast` is outside code
        std::ptr::from_ref(self).cast()
    }

    fn outer(&self) {
        let p = self.erased();
        self.inner(p);
    }

    fn inner(&self, p: *const ()) {
        let heap = unsafe { &mut **self.heap.get() };

        // accepted: neither argument shows the struct, but `call` runs
        // `poke(p)`, which calls `alloc`
        other_crate::call(poke, p);
    }
}

fn poke(p: *const ()) {
    unsafe { (*(p as *const Ctx)).alloc(1) };
}
```

## Towards a general solution

Both causes start the same way: a function that holds no cell hands the struct to code outside the crate, which stores the pointer or erases its type. A more general rule is worth exploring. One candidate is to never let the struct be handed to code outside the crate, in any function. It would reject every example above. It is not adopted yet: the VM has three places today where the struct must be handed over, such as the call into a native function, and each would need an exception with its own argument.

## What the check trusts

The check also trusts two things it does not examine: that the anchor function has the right signature, and that code in other crates cannot name a cell, which is why cell fields must not be `pub`.

# Appendix: prior work

Both limitations are well known in the Rust community, and so is the trouble they cause for context-like objects.

## The two limitations

- **Partial borrows.** Niko Matsakis described the problem in ["After NLL: Interprocedural conflicts"](https://smallcultfollowing.com/babysteps/blog/2018/11/01/after-nll-interprocedural-conflicts/) (2018) and proposed a language fix, [view types](https://smallcultfollowing.com/babysteps/blog/2021/11/05/view-types/), in which a function declares the fields it touches. View types are on the [2026 borrow checker roadmap](https://rust-lang.github.io/goals/2026/roadmap-borrow-checker-within.html) but not yet designed in full. Crates such as [`borrow`](https://docs.rs/borrow) emulate them with macros.
- **Taking turns.** The standard library offers `Cell` (no references handed out, so every access is one step) and `RefCell` (turns checked at run time). [GhostCell](https://plv.mpi-sws.org/rustbelt/ghostcell/) (ICFP 2021) checks turns at compile time: access to a cell requires lending a separate token, and the borrow checker enforces turns on the token.

## Contexts

- **Game development.** ["Leaving Rust gamedev after 3 years"](https://loglog.games/blog/leaving-rust-gamedev/) (LogLog Games, 2024) is the best-known account of a context struct passed down the call chain fighting the borrow checker. The usual advice, to split the struct up, is what a context cannot follow.
- **Garbage-collected heaps.** Handing out handles while still allocating is a known hard case. Manish Goregaokar's ["A Tour of Safe Tracing GC Designs in Rust"](https://manishearth.github.io/blog/2021/04/05/a-tour-of-safe-tracing-gc-designs-in-rust/) surveys the designs, and [`safe-gc`](https://fitzgen.com/2024/02/06/safe-gc.html) shows the index-based alternative that needs no `unsafe`.
- **Arenas.** `bumpalo`'s `Bump::alloc(&self, value) -> &mut T` is the same pattern as ours: `&self` in, `&mut` out, cells inside.

## Where the cost can go

The obligation that borrows take turns has to be discharged somewhere. Across all of the above there are only three places:

| Who pays | Mechanism | Cost |
|---|---|---|
| The caller | `&mut self`, or GhostCell tokens | Every signature carries the permission |
| Nobody at compile time | `RefCell` | A check on every access, and a panic instead of a compile error |
| The implementation | `UnsafeCell` | `unsafe` code, argued by hand |

The compiler checks one function at a time, from signatures alone. So a permission it can check must appear in the signatures the caller sees, and one hidden from the caller is hidden from the compiler too.

We chose the third row, because a context has few implementors and many callers. What this document adds to it is the automatic check: we found no earlier lint or other static check for the hand-argued `UnsafeCell` route. The closest relatives are Clippy's `await_holding_refcell_ref`, a rule of the same style for `RefCell`, and the `pin!` and [`generativity`](https://docs.rs/generativity) macros, which tie a lifetime to a local the way the anchor does.
