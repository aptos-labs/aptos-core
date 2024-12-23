# 单元测试 (Unit Tests)

Unit testing for Move adds three new annotations to the Move source language:

Move 语言中存在三种单元测试标注：

* `#[test]`
* `#[test_only]`, and
* `#[expected_failure]`.

They respectively mark a function as a test, mark a module or module member (`use`, function, or struct) as code to be included for testing only, and mark that a test is expected to fail. These annotations can be placed on a function with any visibility. Whenever a module or module member is annotated as `#[test_only]` or `#[test]`, it will not be included in the compiled bytecode unless it is compiled for testing.

它们分别把函数、模块或模块成员（`use` 声明，函数 function，或结构体 struct）标记为只用于测试的代码，同时也标记期望失败的测试。这些标注可以用在任何可见性（visibility）函数上。无论何种情况，被标注为 `#[test_only]` 或 `#[test]` 的模块或模块成员除非用于测试，其它情况都不会被编译成字节码。

## 测试注解：含义和使用方法（Testing Annotations: Their Meaning and Usage）

Both the `#[test]` and `#[expected_failure]` annotations can be used either with or without arguments.

`#[test]` 和 `#[expected_failure]` 两个注解均可以在有、无参数情况下使用。

Without arguments, the `#[test]` annotation can only be placed on a function with no parameters. This annotation simply marks this function as a test to be run by the unit testing harness.

没有参数的 `#[test]` 标记只能用于没有参数的函数。表示该函数作为单元测试函数被运行。

```
#[test] // 正确 // OK
fun this_is_a_test() { ... }

#[test] // 编译失败，因为函数需要参数 // Will fail to compile since the test takes an argument
fun this_is_not_correct(arg: signer) { ... }
```

A test can also be annotated as an `#[expected_failure]`. This annotation marks that the test should is expected to raise an error. You can ensure that a test is aborting with a specific abort code by annotating it with `#[expected_failure(abort_code = <code>)]`, if it then fails with a different abort code or with a non-abort error the test will fail. Only functions that have the `#[test]` annotation can also be annotated as an #`[expected_failure]`.

测试也可以使用 `#[expected_failure]` 标注，表示该函数会抛出错误。你可以使用 `#[expected_failure(abort_code = <code>)]` 这种方式方式确保此测试会被指定错误码打断，如果抛出不同错误码或没有抛出错误测试将失败。只有被 `#[test]` 标注的函数才能使用 `#[expected_failure]` 标注。

```
#[test]
#[expected_failure]
public fun this_test_will_abort_and_pass() { abort 1 }

#[test]
#[expected_failure]
public fun test_will_error_and_pass() { 1/0; }

#[test]
#[expected_failure(abort_code = 0)]
public fun test_will_error_and_fail() { 1/0; }

#[test, expected_failure] // 可以合并多个属性。测试将会通过。 // Can have multiple in one attribute. This test will pass.
public fun this_other_test_will_abort_and_pass() { abort 1 }
```

With arguments, a test annotation takes the form `#[test(<param_name_1> = <address>, ..., <param_name_n> = <address>)]`. If a function is annotated in such a manner, the function's parameters must be a permutation of the parameters <`param_name_1>, ..., <param_name_n>`, i.e., the order of these parameters as they occur in the function and their order in the test annotation do not have to be the same, but they must be able to be matched up with each other by name.

测试标注可以采用 `#[test(<param_name_1> = <address>, ..., <param_name_n> = <address>)]` 这种形式指定参数。如果函数使用这样的标注，函数的参数则必须为 `<param_name_1>, ..., <param_name_n>` 的形式。参数在函数中的顺序不必与注解中顺序一致，但必须要能根据参数名匹配。

Only parameters with a type of `signer` are supported as test parameters. If a non-`signer` parameter is supplied, the test will result in an error when run.

只有 `signer` 类型可以用作测试参数。使用非 `signer` 类型参数，测试将会失败。

```
#[test(arg = @0xC0FFEE)] // 正确 // OK
fun this_is_correct_now(arg: signer) { ... }

#[test(wrong_arg_name = @0xC0FFEE)] // 不正确: 参数名不匹配 // Not correct: arg name doesn't match
fun this_is_incorrect(arg: signer) { ... }

#[test(a = @0xC0FFEE, b = @0xCAFE)] // 正确，多参数情况下必须为每个参数提供值。 // OK. We support multiple signer arguments, but you must always provide a value for that argument
fun this_works(a: signer, b: signer) { ... }

// 在某处声明一个命名地址（named address） // somewhere a named address is declared
#[test_only] // 命名地址支持 test-only 注解 // test-only named addresses are supported
address TEST_NAMED_ADDR = @0x1;
...
#[test(arg = @TEST_NAMED_ADDR)] // 支持命名地址！ // Named addresses are supported!
fun this_is_correct_now(arg: signer) { ... }
```

An expected failure annotation can also take the form `#[expected_failure(abort_code = <u64>)]`. If a test function is annotated in such a way, the test must abort with an abort code equal to `<u64>`. Any other failure or abort code will result in a test failure.

预期失败的标注使用 `#[expected_failure(abort_code = <u64>)]` 这种形式。如果函数被这样标注，测试错误码必须为 `<u64>`。任何其它的错误或错误码都会失败。

```
#[test, expected_failure(abort_code = 1)] // 这个测试会失败 // This test will fail
fun this_test_should_abort_and_fail() { abort 0 }

#[test]
#[expected_failure(abort_code = 0)] // 这个测试会通过 // This test will pass
fun this_test_should_abort_and_pass_too() { abort 0 }
```

A module and any of its members can be declared as test only. In such a case the item will only be included in the compiled Move bytecode when compiled in test mode. Additionally, when compiled outside of test mode, any non-test `use`s of a `#[test_only]` module will raise an error during compilation.

模块和它的成员可以被声明为仅测试用。这种情况它们只会在测试模式下编译。此外，在非测试模式下，任何被 `#[test_only]` 标记的模块都会在编译时报错。

```
#[test_only] // test only 属性可以用于模块  // test only attributes can be attached to modules
module abc { ... }

#[test_only] // test only 属性可以用于命名地址 // test only attributes can be attached to named addresses
address ADDR = @0x1;

#[test_only] // .. 用于 use 声明 // .. to uses
use 0x1::some_other_module;

#[test_only] // .. 用于结构体 // .. to structs
struct SomeStruct { ... }

#[test_only] // .. 用于函数。只能在测试函数中调用，但自身不是测试 // .. and functions. Can only be called from test code, but not a test
fun test_only_function(...) { ... }
```

## 运行单元测试（Running Unit Tests）

Unit tests for a Move package can be run with the [`move test`
command](./packages.md).

使用 [`move test` 命令](./packages.md)运行包中的单元测试。

When running tests, every test will either `PASS`, `FAIL`, or `TIMEOUT`. If a test case fails, the location of the failure along with the function name that caused the failure will be reported if possible. You can see an example of this below.

运行测试的结果包括 `PASS`、`FAIL` 或 `TIMEOUT`。如果测试失败，将会尽可能的提供执行失败的位置及函数名信息。请看下面的例子。

A test will be marked as timing out if it exceeds the maximum number of instructions that can be executed for any single test. This bound can be changed using the options below, and its default value is set to 5000 instructions. Additionally, while the result of a test is always deterministic, tests are run in parallel by default, so the ordering of test results in a test run is non-deterministic unless running with only one thread (see `OPTIONS` below).

任何测试执行超过最大数量指令限制将会标记成超时。可以通过参数调整此限制，默认值为 5000 条指令。此外，虽然测试结果是确定的，但由于测试默认并行执行，所以测试结果的顺序是不确定的，除非使用单线程模式（见下述参数）。

There are also a number of options that can be passed to the unit testing binary to fine-tune testing and to help debug failing tests. These can be found using the help flag:

存在大量参数细粒度调整测试工具的行为，帮助调试失败的测试。可以通过 help 参数查看。

```
$ move -h
```

## 示例（Example）

A simple module using some of the unit testing features is shown in the following example:

下面例子展示了一个简单的使用了单元测试特性的模块：

First create an empty package and change directory into it:

首先创建一个空 package 进入目录：

```
$ move new TestExample; cd TestExample
```

Next add the following to the `Move.toml`:

接下来添加下面内容到 `Move.toml` 文件：

```
[dependencies]
MoveStdlib = { git = "https://github.com/diem/diem.git", subdir="language/move-stdlib", rev = "56ab033cc403b489e891424a629e76f643d4fb6b", addr_subst = { "std" = "0x1" } }
```

Next add the following module under the `sources` directory:

接下来在 `sources` 目录下添加下述模块：

```
// 文件路径: sources/my_module.move // filename: sources/my_module.move
module 0x1::my_module {

    struct MyCoin has key { value: u64 }

    public fun make_sure_non_zero_coin(coin: MyCoin): MyCoin {
        assert!(coin.value > 0, 0);
        coin
    }

    public fun has_coin(addr: address): bool {
        exists<MyCoin>(addr)
    }

    #[test]
    fun make_sure_non_zero_coin_passes() {
        let coin = MyCoin { value: 1 };
        let MyCoin { value: _ } = make_sure_non_zero_coin(coin);
    }

    #[test]
    // 如果不关心错误码也可以使用 #[expected_failure] // Or #[expected_failure] if we don't care about the abort code
    #[expected_failure(abort_code = 0)]
    fun make_sure_zero_coin_fails() {
        let coin = MyCoin { value: 0 };
        let MyCoin { value: _ } = make_sure_non_zero_coin(coin);
    }

    #[test_only] // 仅用作测试的帮助方法 // test only helper function
    fun publish_coin(account: &signer) {
        move_to(account, MyCoin { value: 1 })
    }

    #[test(a = @0x1, b = @0x2)]
    fun test_has_coin(a: signer, b: signer) {
        publish_coin(&a);
        publish_coin(&b);
        assert!(has_coin(@0x1), 0);
        assert!(has_coin(@0x2), 1);
        assert!(!has_coin(@0x3), 1);
    }
}
```

### 运行测试（Running Tests）

You can then run these tests with the `move test` command:

你可以使用 `move test` 命令运行测试。

```
$ move test
BUILDING MoveStdlib
BUILDING TestExample
Running Move unit tests
[ PASS    ] 0x1::my_module::make_sure_non_zero_coin_passes
[ PASS    ] 0x1::my_module::make_sure_zero_coin_fails
[ PASS    ] 0x1::my_module::test_has_coin
Test result: OK. Total tests: 3; passed: 3; failed: 0
```

### 使用测试参数（Using Test Flags）

#### `-f <str>` 或 `--filter <str>`（`-f <str>` or `--filter <str>`）

This will only run tests whose fully qualified name contains `<str>`. For example if we wanted to only run tests with `"zero_coin"` in their name:

仅运行名字包含 `<str>` 字符的测试。例如只想运行名字包含 `"zero_coin"` 的测试：


```
$ move test -f zero_coin
CACHED MoveStdlib
BUILDING TestExample
Running Move unit tests
[ PASS    ] 0x1::my_module::make_sure_non_zero_coin_passes
[ PASS    ] 0x1::my_module::make_sure_zero_coin_fails
Test result: OK. Total tests: 2; passed: 2; failed: 0
```

#### `-i <bound>` 或 `--gas_used <bound>`（`-i <bound>` or `--gas_used <bound>`）

This bounds the amount of gas that can be consumed for any one test to `<bound>`:

调整测试指令限制为 `<bound>`：

```
$ move test -i 0
CACHED MoveStdlib
BUILDING TestExample
Running Move unit tests
[ TIMEOUT ] 0x1::my_module::make_sure_non_zero_coin_passes
[ TIMEOUT ] 0x1::my_module::make_sure_zero_coin_fails
[ TIMEOUT ] 0x1::my_module::test_has_coin

Test failures:

Failures in 0x1::my_module:

┌── make_sure_non_zero_coin_passes ──────
│ Test timed out
└──────────────────


┌── make_sure_zero_coin_fails ──────
│ Test timed out
└──────────────────


┌── test_has_coin ──────
│ Test timed out
└──────────────────

Test result: FAILED. Total tests: 3; passed: 0; failed: 3
```

#### `-s` 或 `--statistics`（`-s` or `--statistics`）

With these flags you can gather statistics about the tests run and report the runtime and gas used for each test. For example, if we wanted to see the statistics for the tests in the example above:

使用此参数你可以得到每个测试的运行报告及执行指令的统计信息。例如查看上述示例的统计数据：

```
$ move test -s
CACHED MoveStdlib
BUILDING TestExample
Running Move unit tests
[ PASS    ] 0x1::my_module::make_sure_non_zero_coin_passes
[ PASS    ] 0x1::my_module::make_sure_zero_coin_fails
[ PASS    ] 0x1::my_module::test_has_coin

Test Statistics:

┌────────────────────────────────────────────────┬────────────┬───────────────────────────┐
│                   Test Name                    │    Time    │   Gas Used   │
├────────────────────────────────────────────────┼────────────┼───────────────────────────┤
│ 0x1::my_module::make_sure_non_zero_coin_passes │   0.009    │             1             │
├────────────────────────────────────────────────┼────────────┼───────────────────────────┤
│ 0x1::my_module::make_sure_zero_coin_fails      │   0.008    │             1             │
├────────────────────────────────────────────────┼────────────┼───────────────────────────┤
│ 0x1::my_module::test_has_coin                  │   0.008    │             1             │
└────────────────────────────────────────────────┴────────────┴───────────────────────────┘

Test result: OK. Total tests: 3; passed: 3; failed: 0
```

#### `-g` 或 `--state-on-error`（`-g` or `--state-on-error`）

These flags will print the global state for any test failures. e.g., if we added the following (failing) test to the `my_module` example:

这个参数会在测试失败情况下打印全局状态。如在 `my_module` 模块中添加下述失败测试：

```
module 0x1::my_module {
    ...
    #[test(a = @0x1)]
    fun test_has_coin_bad(a: signer) {
        publish_coin(&a);
        assert!(has_coin(@0x1), 0);
        assert!(has_coin(@0x2), 1);
    }
}
```

we would get the following output when running the tests:

当运行测试时我们将得到下面的输出：

```
$ move test -g
CACHED MoveStdlib
BUILDING TestExample
Running Move unit tests
[ PASS    ] 0x1::my_module::make_sure_non_zero_coin_passes
[ PASS    ] 0x1::my_module::make_sure_zero_coin_fails
[ PASS    ] 0x1::my_module::test_has_coin
[ FAIL    ] 0x1::my_module::test_has_coin_bad

Test failures:

Failures in 0x1::my_module:

┌── test_has_coin_bad ──────
│ error[E11001]: test failure
│    ┌─ /home/tzakian/TestExample/sources/my_module.move:47:10
│    │
│ 44 │      fun test_has_coin_bad(a: signer) {
│    │          ----------------- In this function in 0x1::my_module
│    ·
│ 47 │          assert!(has_coin(@0x2), 1);
│    │          ^^^^^^^^^^^^^^^^^^^^^^^^^^ Test was not expected to abort but it aborted with 1 here
│
│
│ ────── Storage state at point of failure ──────
│ 0x1:
│       => key 0x1::my_module::MyCoin {
│           value: 1
│       }
│
└──────────────────

Test result: FAILED. Total tests: 4; passed: 3; failed: 1
```
