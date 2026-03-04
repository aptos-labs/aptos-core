# Function Values in the Move Prover

With functions becoming values in Move 2.2, a semantic model is required how to specify and verify them. Formal verification with higher-order functions which have side-effects (like the ones in Move or Rust) is challenging: as with most aspects of formal verification of interesting systems, the problems are generally undecidable. However, there are lots of cases where, via the intuitionistic-logical approach used in the Move prover, meaningful properties can be verified. 

This note discusses and documents the (evolving) implementation of function values in the  Prover. We first look at the Move execution semantics, and then how to write specifications and verify them.

# Execution Semantics

For modeling the semantics of execution in the Prover, we need to describe the `PackClosure` and the `CallClosure` instructions.  The implementation for this already landed a while ago in the Move Prover and is in use, below is how it works.

## Collecting the Closures in the Program

The approach makes use of a general principal which is also used for monomorphization and global invariant verification of the Prover as described in the TACAS’22 paper: we only need to look at the *final fragment of code which is under verification*, specifically if it comes to package private functionality.  

For every function type `|T|R` in the program, we collect all the ways how a closure of this type is constructed. Then we build an abstract data type to represent `|T|R` with a variant for each closure found in the program.

Lets assume the program has functions `f(T):R` and `g(S,T):R`. Moreover, there are two closures in the program, namely `|x| f(x)` and `|x| g(s,x)`. The type `|T|R` is represented as:

```rust
enum Func_T_R {
  Closure_f,
  Closure_g(S),
  Unknown(u64)  // There exists many unknown functions, counted by the `u64`
}  
```

## Evaluating a Function Value

Now that there is a representation of function values via the `Func_T_R` type, calling a closure via instruction `CallClosure` is described as follows:

```rust
fun call_Func_T_R(f: Func_T_R, x: T): R {
  match(f) {
    Closure_f    => f(x),
    Closure_g(s) => g(s, x),
    Unknown(_)   => havoc // mut ref param, abort_flag, global state
  }
}    
```

Notice that in many cases, the Prover will be able to simplify the match to one of the concrete functions used in the program. Consider, for example a call like `call(|x| g(s, x))`. By argument propagation into the verification context, the prover knows that its calling `Closure_g` and can reduce to the verification of the direct call. This give’s similar verification power as today already exist for inline functions.

However, when there is no context from argument propagation, the semantics of the `Unknown` function variant leaves an arbitrarily unconstraint behavior of the function value in place. Here is where the *specification semantics* kicks in.

# Specification Semantics

## Functions in MSL

Even though functions are just values for Move’s execution model, in the specification world they are something special: specs *are* *about* describing how functions behave. A function in MSL is described by the following clauses, where for simplicity, we assume single parameters and results, and each clause is optional:

```rust
fun f(x: X): Y { .. }
spec f {
  modifies global<R>(E[x]); // E[x]: address
  requires P[a, x];
  aborts_if A[a, x];
  ensures Q[a, x, result]
}
```

Semantically, a function specification is a relation between inputs, outputs, and pre and post states of the function execution. Hereby, a state is constituted by the resources which are available to the program.

## Lifting Function Characteristics to Predicates

In order to reason about function values, we need to be able to access the same components known already from specification blocks of functions for *function parameters.* Consider the following  Move function:

```rust
fun call_twice(f: |T|T, x: T): T { 
    f(f(x)) 
}
```

The behavior of `call_twice` can be specified if we can access the post-condition of `f` as a predicate by itself:

```rust
spec call_twice {
   // ensures<f>(params) is the post-condition of f
   ensures exists t: T: ensures<f>(x, t) ==> ensures<f>(t, result);
}
```

Notice how this uses an intermediate value `t` to connect the post-condition of `f` with the input of calling `f` a second time. In a similar style, we can propagate the aborts condition of `f`:

```rust
spec call_twice {
   aborts_if aborts_if<f>(x);
   aborts_if exists t: T: ensures<f>(x, t) ==> aborts_if<f>(t);
}
```

However, there is one issue in this representation: what if `f` depends on global state, for example, increments a counter in a resource? In order to allow state the relation between input and output states of `f` need to be made explicit in the spec. This requires a new construct in the specification language, so called  *state labels.* State labels allow name pre and post state of predicate evaluation:

```rust
spec call_twice {
   aborts_if aborts_if<f>(x);
   aborts_if exists t: T: ensures<f>(x, t)@S ==> S@aborts_if<f>(t);
}
```

Here `S` is used to capture the post-state of the ensures condition, which is matched with the pre-state for the aborts condition. 

Notice that state labels are technically already in the Prover. Namely the `old(e)` expression in specifications is based on capturing the pre-state with an internal label. Moreover, when the prover composes opaque function specifications at call side, it also uses labels to capture state for intermediate steps. Thus the concept of labels does not produce new complexities.

# Verification Semantics

The following new builtin predicates are introduced, where `f` must be a name of function value (either an existing function or a name of local with function type):

```rust
requires<f>(x)       // denotes the pre-condition of `f`
aborts_if<f>(x)      // denotes the aborts condition of `f`
ensures<f>(x, y)     // denotes the post condition of `f`
modifies<f>(x)       // denotes the set of modify clauses of `f`
```

Those predicates can be combined with state labels which allow to associate pre and post states of invocations of function `f`:

```rust
S@requires<f>(x)     // denotes precondition in given prestate S
ensures<f>(x)@R      // denotes postcondition in given poststate R
S@ensures<f>(x)@R    // denotes both pre- and poststates S and R
```

## Verifying Definition Side

When verifying a function with function value parameters `f`, the notation `S@ensures<f>(x)@R` is associated with an *uninterpreted function* of a matching type:

```rust
spec type Storage; 
spec fun _@ensures<f>(_)@_(S: Storage, x: InOut, R: Storage): bool
```

Notice that already today, in the Prover, specification functions take state of individual resources as a parameter. The type `Storage` goes behind this, as *all* resources are contained in it. However, as least in the Boogie encoding, this should not cause trouble; the existing resources of the verified program would be put into a tuple to make up a `Storage`  val. 

Returning to the `call_twice` example, this uninterpreted function is injected into the code as below:

```rust
fun call_twice(f: |T|T, x: T): T {
  capture S1;
	let t = f(x);
	capture S2;
	assume S1@ensures<f>(x, t)@S2;
	let result = f(t);
	capture S3;
	assume S2@ensures<f>(t, result)@S3;
	// The below assert will be introduced from the spec block
	assert exists t: T: S1@ensures<f>(x, t)@S2 => S2@ensures<f>(t, result)@S3;
	result 
}
```

## Verifying Caller Side (Non-Opaque)

When verifying a function which is non-opaque (the default), the Prover essentially inlines the function definition at the caller side. The following example illustrates the code which is actually verified:

```rust
call_twice(increment, 0) 
-->  
{ 
    let result = increment(increment(0));
    assert 
       exists t: T: ensures<increment>(0, t)@S: S@ensures<increment>(t, result);
    result
} 
```

Notice that with the predicate `ensures<increment>(x, result)` the post-condition of a *concrete function* (or a function resulting from lambda lifting) is requested. This is in difference to the *uninterpreted* function at definition side. If `increment` is opaque, the existing spec would be inserted:

```rust
fun increment(x: u64): u64 { x + 1 }
spec increment { pragma opaque; ensures result == x + 1; }

// ensures<increment>(x, result) <==> result == x + 1
```

If the function `increment` has no specification, we will derive one from the code. This can be done we weakest precondition or deductive compilation. It is also possible to introduce an uninterpreted function and let coincide with the function body, as shown below:

```rust
fun increment(x: u64):u64 {
   capture S;
   let result = x + 1;
   assume S@ensures<increment>(x, result)@S; 
   result
}
```

Notice that if we define `ensures<increment>` via an assume as an above, the solution the Prover finds can be arbitrary, as long as it satisfies the given predicate. 

## Verifying Caller Side (Opaque)

For the opaque case, we instantiate pre/post conditions of the `call_twice` function, as usual. Assuming `call_twice` is opaque, then `call_twice(increment, 0)` at caller side will result in below. Notice that we must: 

```rust
{
    let havoc result;
    assert exists S: Storage: exists t: T: 
       ensures<increment>(0, t)@S ==> S@ensures<increment>(t, result);
    result
}
```

The same mechanism as described above for determining `ensures<increment>` is used to verify this.

## Verifying Requirements on Functions

What happens if the function passed to another function is expected to satisfy some properties? For example, we can expect the `call_twice` argument to not abort. This would be written as follows:

```rust
spec call_twice {
   pragma opaque;
   requires !aborts_if<f>(x); // passed function is not allowed to abort
   aborts_if false;           // ... then this function does not abort as well
}
```

This translates as follows at caller side:

```rust
call_twice(increment, 0) 
-->  
{
    assert forall S: Storage, x: T: !S@aborts_if<increment>(x)
}
```

Notice that this condition is trivial to prove if `increment` comes with an `aborts_if false` specification clause. It can arbitrary hard in other cases because of universal quantification over storage and parameters.

## Dealing with the Modifies Clause

In the discussion until here, the `modifies` clause was ignored. It works as follows.

In general `S@modifies<f>(r1, .., rn)@R` is a predicate where each expression `ri` denotes a resource, as in `R[x]`. This maps the meaning down to the following formula:

```rust
S@modifies<f>(r1, .., rn) <==>
	forall r: RESOURCES: (!exists i: r == ri) ==> S[r] == R[r]
```

Thus for every resource which is not one of the given ones, pre and post state are the same.

Note the notation `S@modifies<f>()@R` specifies that no modifications are possible. This can be shortcut as `S@pure<f>@R`. 

Here is how this plays out for the `call_twice` function, where we only allow the function `f` to modify a particular `Counter` resource:

```rust
spec call_twice {
   // Ensure that f will only modifier Counter[addr]
   ensures modifies<f>(Counter[addr]);  
}
```
