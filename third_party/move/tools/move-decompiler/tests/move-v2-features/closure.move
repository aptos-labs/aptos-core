module 0x99::basic_enum {
    #[persistent]
    fun increment_by_one(x: &mut u64): u64 { *x = *x + 1; *x }

    enum FV<T> has key {
        V1 { v1: |&mut T|T has copy+store},
    }

    fun test_fun_vec(s: &signer) {
        // ok case
        let f1: |&mut u64|u64 has copy+store = increment_by_one;
        let v1 = FV::V1{v1: f1};
        move_to(s, v1);
    }
}

module 0x99::basic_struct {
  struct Wrapper<T> has key {
    fv: T
  }

  #[persistent]
   fun test(f: &||u64): u64 {
    if (f == f)
        1
    else
        2
   }

  // all abilities satisfied
  fun add_resource_with_struct(acc: &signer, f: | &||u64 |u64 has copy+store+drop) {
    move_to<Wrapper<| &||u64 |u64 has copy+store+drop>>(acc, Wrapper { fv: f});
  }

  public fun test_driver(acc: &signer){
    // ok case
    let f: | &||u64 |u64 has copy+store+drop = test;
    add_resource_with_struct(acc, f);
  }
}

module 0x99::nested_lambda {

    /// A higher order function on ints
    fun map1(x: u64, f: |u64|u64): u64 {
        f(x)
    }

    /// Another higher order function on ints
    fun map2(x: u8, f: |u8|u8): u8 {
        f(x)
    }

    /// A tests which nests things
    fun nested(x: u64, c: u64): u64 {
        map1(x, |y| (map2((y - c as u8), |y| y + (c as u8)) as u64))
    }
}


module 0x99::lambda_basic {
    /// A higher order function on ints
    fun map(x: u64, f: |u64|u64 has drop): u64 {
        f(x)
    }

    /// Tests basic usage, without name overlap
    fun no_name_clash(x: u64, c: u64): u64 {
        map(x, |y| y + c)
    }

    /// Basic usage in the presence of name clask
    fun with_name_clash1(x: u64, c: u64): u64 {
        map(x, |x| x + c)
    }

    /// More clashes
    fun with_name_clash2(x: u64, c: u64): u64 {
        map(x, |x| {
            let x = c + 1;
            x
        } + x)
    }
}

module 0x99::lambda_pattern {

    /// Test struct
    struct S<T> {
        x: T
    }

    /// A higher order function on `S`
    fun consume<T>(s: S<T>, x: T, f: |S<T>, T|T): T {
        f(s, x)
    }

    /// Lambda with pattern
    fun pattern(s: S<u64>, x: u64): u64 {
        consume(s, x, |S{x}, _y| { let y = x; x + y})
    }
}

module 0x99::lambda_fun_wrapper {
    struct Work(|u64|u64) has drop;

    fun take_work(_work: Work) {}

    fun t1():bool {
        let work = Work(|x| x + 1);
        work == (|x| x + 2)
    }

    fun t2() {
        take_work(|x| x + 1)
    }
}

module 0x99::lambda_no_param {
    inline fun foo(f:|u64, u64| u64, g: |u64, u64| u64, x: u64, _y: u64): u64 {
        f(x, _y) + g(x, _y)
    }

    public fun test() {
        assert!(foo(|_, _| 3, |_, _| 10, 10, 100) == 13, 0);
    }
}

module 0x99::lambda_no_param1 {
    fun foo(f:|u64, u64| u64, g: |u64, u64| u64, x: u64, _y: u64): u64 {
        f(x, _y) + g(x, _y)
    }

    public fun test() {
        assert!(foo(|x, _| x, |_, y| y, 10, 100) == 110, 0);
    }
}

module 0x99::lambda_inline {
    inline fun foo(f: |&u64|) {
    }

    fun g() {
        foo(|v| {
            v == &1;
        });
    }
}

module 0x99::lambda_inline1 {
    inline fun foo(f:|u64, u64, u64| u64, g: |u64, u64, u64| u64, x: u64, _: u64, y: u64, z: u64): u64 {
	let r1 = f({x = x + 1; x}, {y = y + 1; y}, {z = z + 1; z});
	let r2 = g({x = x + 1; x}, {y = y + 1; y}, {z  = z + 1 ; z});
	r1 + r2 + 3*x + 5*y + 7*z
    }

    public fun test() {
	let r = foo(|x, _, z| x*z, |_, y, _| y, 1, 10, 100, 1000);
        assert!(r == 9637, r);
    }
}

module 0x99::lambda_arg {
    fun foo(f:|u64| u64, x: u64): u64 {
        f(x)
    }

    public fun test(): u64 {
        foo(|_| 3, 10)
    }

    public fun main() {
        assert!(test() == 3, 5);
    }
}

module 0x99::lambda_generics {

    struct S<T> has drop { x: T }


    fun inlined<T:drop>(f: |S<T>|S<T>, s: S<T>) {
        f(s);
    }

    fun id<T>(self: S<T>): S<T> {
        self
    }

    fun test_receiver_inference(s: S<u64>) {
        // In the lambda the type of `s` is not known when the expression is checked,
        // and the receiver function `id` is resolved later when the parameter type is unified
        // with the lambda expression
        inlined(|s| s.id(), s)
    }
}
