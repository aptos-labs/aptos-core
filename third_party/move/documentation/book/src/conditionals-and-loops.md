# Conditionals and Loops

## Conditionals

An `if` expression specifies that some code should only be evaluated if a certain condition is true. For example:

```move
script {
  fun example() {
    if (x > 5) x = x - 5
  }
}
```

The condition must be an expression of type `bool`.

An `if` expression can optionally include an `else` clause to specify another expression to evaluate when the condition is false.

```move
script {
  fun example() {
    if (y <= 10) y = y + 1 else y = 10
  }
}
```

Either the "true" branch or the "false" branch will be evaluated, but not both. Either branch can be a single expression or an expression block.

The conditional expressions may produce values so that the `if` expression has a result.

```move
script {
  fun example() {
    let z = if (x < 100) x else 100;
  }
}
```

The expressions in the true and false branches must have compatible types. For example:

```move
script {
  fun example() {
    // x and y must be u64 integers
    let maximum: u64 = if (x > y) x else y;

    // ERROR! branches different types
    let z = if (maximum < 10) 10u8 else 100u64;

    // ERROR! branches different types, as default false-branch is () not u64
    if (maximum >= 10) maximum;
  }
}
```

If the `else` clause is not specified, the false branch defaults to the unit value. The following are equivalent:

```move
script {
  fun example() {
    if (condition) true_branch // implied default: else ()
    if (condition) true_branch else ()
  }
}
```

Commonly, `if` expressions are used in conjunction with expression blocks.

```move
script {
  fun example() {
    let maximum = if (x > y) x else y;
    if (maximum < 10) {
        x = x + 10;
        y = y + 10;
    } else if (x >= 10 && y >= 10) {
        x = x - 10;
        y = y - 10;
    }
  }
}

```

### Grammar for Conditionals

> _if-expression_ → **if (** _expression_ **)** _expression_ _else-clause_<sub>_opt_</sub>

> _else-clause_ → **else** _expression_

## While, For, and Loop

Move offers three constructs for looping: `while`, `for`, and `loop`.

### `while` loops

The `while` construct repeats the body (an expression of type unit) until the condition (an expression of type `bool`) evaluates to `false`.

Here is an example of a simple `while` loop that computes the sum of the numbers from `1` to `n`:

```move
script {
  fun sum(n: u64): u64 {
    let sum = 0;
    let i = 1;
    while (i <= n) {
      sum = sum + i;
      i = i + 1
    };

    sum
  }
}
```

Infinite loops are allowed:

```move
script {
  fun foo() {
    while (true) { }
  }
}
```

#### `break`

The `break` expression can be used to exit a loop before the condition evaluates to `false`. For example, this loop uses `break` to find the smallest factor of `n` that's greater than 1:

```move
script {
  fun smallest_factor(n: u64): u64 {
    // assuming the input is not 0 or 1
    let i = 2;
    while (i <= n) {
      if (n % i == 0) break;
      i = i + 1
    };

    i
  }
}
```

The `break` expression cannot be used outside of a loop.

#### `continue`

The `continue` expression skips the rest of the loop and continues to the next iteration. This loop uses `continue` to compute the sum of `1, 2, ..., n`, except when the number is divisible by 10:

```move
script {
  fun sum_intermediate(n: u64): u64 {
    let sum = 0;
    let i = 0;
    while (i < n) {
      i = i + 1;
      if (i % 10 == 0) continue;
      sum = sum + i;
    };

    sum
  }
}
```

The `continue` expression cannot be used outside of a loop.

#### The type of `break` and `continue`

`break` and `continue`, much like `return` and `abort`, can have any type. The following examples illustrate where this flexible typing can be helpful:

```move
script {
  fun pop_smallest_while_not_equal(
    v1: vector<u64>,
    v2: vector<u64>,
  ): vector<u64> {
    let result = vector::empty();
    while (!vector::is_empty(&v1) && !vector::is_empty(&v2)) {
      let u1 = *vector::borrow(&v1, vector::length(&v1) - 1);
      let u2 = *vector::borrow(&v2, vector::length(&v2) - 1);
      let popped =
        if (u1 < u2) vector::pop_back(&mut v1)
        else if (u2 < u1) vector::pop_back(&mut v2)
        else break; // Here, `break` has type `u64`
      vector::push_back(&mut result, popped);
    };

    result
  }
}
```

```move
script {
  fun pick(
    indexes: vector<u64>,
    v1: &vector<address>,
    v2: &vector<address>
  ): vector<address> {
    let len1 = vector::length(v1);
    let len2 = vector::length(v2);
    let result = vector::empty();
    while (!vector::is_empty(&indexes)) {
      let index = vector::pop_back(&mut indexes);
      let chosen_vector =
        if (index < len1) v1
        else if (index < len2) v2
        else continue; // Here, `continue` has type `&vector<address>`
      vector::push_back(&mut result, *vector::borrow(chosen_vector, index))
    };

    result
  }
}
```

### The `for` expression

The `for` expression iterates over a range defined using integer-typed `lower_bound` (inclusive) and `upper_bound` (non-inclusive) expressions, executing its loop body for each element of the range. `for` is designed for scenarios where the number of iterations of a loop is determined by a specific range.

Here is an example of a `for` loop that computes the sum of the elements in a range from `0` to `n-1`:

```move
script {
  fun sum(n: u64): u64 {
    let sum = 0;
    for (i in 0..n) {
      sum = sum + i;
    };

    sum
  }
}
```

The loop iterator variable (`i` in the above example) currently must be a numeric type (inferred from the bounds), and the bounds `0` and `n` here can be replaced by arbitrary numeric expressions. Each is only evaluated once at the start of the loop. The iterator variable `i` is assigned the `lower_bound` (in this case `0`) and incremented after each loop iteration; the loop exits when the iterator `i` reaches or exceeds `upper_bound` (in this case `n`).

#### `break` and `continue` in `for` loops

Similar to `while` loops, the `break` expression can be used in `for` loops to exit prematurely. The `continue` expression can be used to skip the current iteration and move to the next. Here's an example that demonstrates the use of both `break` and `continue`. The loop will iterate through numbers from `0` to `n-1`, summing them up. It will skip numbers that are divisible by `3` (using `continue`) and stop when it encounters a number greater than `10` (using `break`):

```move
script {
  fun sum_conditional(n: u64): u64 {
    let sum = 0;
    for (iter in 0..n) {
      if (iter > 10) {
        break; // Exit the loop if the number is greater than 10
      };
      if (iter % 3 == 0) {
        continue; // Skip the current iteration if the number is divisible by 3
      };

      sum = sum + iter;
    };

    sum
  }
}
```

### The `loop` expression

The `loop` expression repeats the loop body (an expression with type `()`) until it hits a `break`

Without a `break`, the loop will continue forever

```move
script {
  fun foo() {
    let i = 0;
    loop { i = i + 1 }
  }
}

```

Here is an example that uses `loop` to write the `sum` function:

```move
script {
  fun sum(n: u64): u64 {
    let sum = 0;
    let i = 0;
    loop {
      i = i + 1;
      if (i > n) break;
      sum = sum + i
    };

    sum
  }
}
```

As you might expect, `continue` can also be used inside a `loop`. Here is `sum_intermediate` from above rewritten using `loop` instead of `while`

```move
script {
  fun sum_intermediate(n: u64): u64 {
    let sum = 0;
    let i = 0;
    loop {
      i = i + 1;
      if (i % 10 == 0) continue;
      if (i > n) break;
      sum = sum + i
    };

    sum
  }
}
```

### The type of `while`, `loop`, and `for` expression

Move loops are typed expressions. The `while` and `for` expressions always have type `()`.

```move
script {
  fun example() {
    let () = while (i < 10) { i = i + 1 };
    let () = for (i in 0..10) {};
  }
}
```

If a `loop` contains a `break`, the expression has type unit `()`

```move
script {
  fun example() {
    (loop { if (i < 10) i = i + 1 else break }: ());
    let () = loop { if (i < 10) i = i + 1 else break };
  }
}
```

If `loop` does not have a `break` or a `continue`, `loop` can have any type much like `return`, `abort`, `break`, and `continue`.

```move
script {
  fun example() {
    (loop (): u64);
    (loop (): address);
    (loop (): &vector<vector<u8>>);
  }
}
```

### Loop Labels

_Since language version 2.1_

A `while` or `loop` statement can have a label that can be referred to by a `break` or `continue` statement. In the presence of nested loops, this allows referring to outer loops. Example:

```move
script {
  fun example(x: u64): u64 {
    'label1: while (x > 10) {
      loop {
        if (x % 2 == 0) {
          x -= 1;
          continue 'label1;
        } else if (x < 10) {
          break 'label1
        } else
          x -= 2
      }
    };
    x
  }
}
```
