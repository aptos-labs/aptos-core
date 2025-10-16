//# run --verbose
script {
  fun main() {
    assert!(0i64 + 0i64 == 0i64, 1000);
    assert!(0i64 + 1i64 == 1i64, 1001);
    assert!(1i64 + 1i64 == 2i64, 1002);
    assert!(0i64 + -1i64 == -1i64, 1003);
    assert!(-1i64 + -1i64 == -2i64, 1004);

    assert!(13i64 + 67i64 == 80i64, 1100);
    assert!(100i64 + 10i64 == 110i64, 1101);
    assert!(-13i64 + -67i64 == -80i64, 1102);
    assert!(-100i64 + -10i64 == -110i64, 1103);

    assert!(0i64 + 9223372036854775807i64 == 9223372036854775807i64, 1200);
    assert!(1i64 + 9223372036854775806i64 == 9223372036854775807i64, 1201);
    assert!(5i64 + 9223372036854775802i64 == 9223372036854775807i64, 1202);
    assert!(0i64 + -9223372036854775808i64 == -9223372036854775808i64, 1203);
    assert!(-1i64 + -9223372036854775807i64 == -9223372036854775808i64, 1204);
    assert!(-5i64 + -9223372036854775803i64 == -9223372036854775808i64, 1205);
  }
}

//# run --verbose
script {
  fun main() {
    1i64 + 9223372036854775807i64; // expect to abort
  }
}

//# run --verbose
script {
  fun main() {
    -1i64 + (-9223372036854775808i64); // expect to abort
  }
}

//# run --verbose
script {
fun main() {
    assert!(0i64 - 0i64 == 0i64, 2000);
    assert!(1i64 - 0i64 == 1i64, 2001);
    assert!(1i64 - 1i64 == 0i64, 2002);
    assert!(-1i64 - -0i64 == -1i64, 2003);
    assert!(-1i64 - -1i64 == 0i64, 2004);

    assert!(52i64 - 13i64 == 39i64, 2100);
    assert!(100i64 - 10i64 == 90i64, 2101);
    assert!(-52i64 - -13i64 == -39i64, 2102);
    assert!(-100i64 - -10i64 == -90i64, 2103);

    assert!(0i64 - -9223372036854775807i64 == 9223372036854775807i64, 2200);
    assert!(1i64 - -9223372036854775806i64 == 9223372036854775807i64, 2201);
    assert!(5i64 - -9223372036854775802i64 == 9223372036854775807i64, 2202);
    assert!(-1i64 - 9223372036854775807i64 == -9223372036854775808i64, 2203);
    assert!(-5i64 - 9223372036854775803i64 == -9223372036854775808i64, 2204);
    assert!(-9223372036854775808i64 - 0 == -9223372036854775808i64, 2205);
}
}

//# run --verbose
script {
  fun main() {
    -2i64 - 9223372036854775807i64; // expect to abort
  }
}

//# run --verbose
script {
  fun main() {
    1i64 - -9223372036854775807i64; // expect to abort
  }
}

//# run --verbose
script {
fun main() {
    assert!(0i64 * 0i64 == 0i64, 3000);
    assert!(1i64 * 0i64 == 0i64, 3001);
    assert!(1i64 * 1i64 == 1i64, 3002);
    assert!(-1i64 * 0i64 == 0i64, 3003);
    assert!(-1i64 * 1i64 == -1i64, 3004);
    assert!(1i64 * -1i64 == -1i64, 3005);
    assert!(-1i64 * -1i64 == 1i64, 3006);

    assert!(6i64 * 7i64 == 42i64, 3100);
    assert!(10i64 * 10i64 == 100i64, 3101);
    assert!(6i64 * -7i64 == -42i64, 3102);
    assert!(10i64 * -10i64 == -100i64, 3103);
    assert!(-6i64 * 7i64 == -42i64, 3104);
    assert!(-10i64 * 10i64 == -100i64, 3105);
    assert!(-6i64 * -7i64 == 42i64, 3106);
    assert!(-10i64 * -10i64 == 100i64, 3107);

    assert!(3037000499i64 * 3037000499i64 == 9223372030926249001i64, 3200);
    assert!(-3037000499i64 * -3037000499i64 == 9223372030926249001i64, 3201);
    assert!(3037000499i64 * -3037000499i64 == -9223372030926249001i64, 3202);
    assert!(-3037000499i64 * 3037000499i64 == -9223372030926249001i64, 3203);
}
}

//# run --verbose
script {
fun main() {
    3037000500i64 * 3037000500i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -3037000500i64 * -3037000500i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    9223372036854775807i64 * 2i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -9223372036854775808i64 * -1i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    assert!(0i64 / 1i64 == 0i64, 4000);
    assert!(1i64 / 1i64 == 1i64, 4001);
    assert!(1i64 / 2i64 == 0i64, 4002);
    assert!(0i64 / -1i64 == 0i64, 4003);
    assert!(1i64 / -1i64 == -1i64, 4004);
    assert!(1i64 / -2i64 == 0i64, 4005);
    assert!(-1i64 / 1i64 == -1i64, 4006);
    assert!(-1i64 / 2i64 == 0i64, 4007);
    assert!(-1i64 / -1i64 == 1i64, 4008);
    assert!(-1i64 / -2i64 == 0i64, 4009);

    assert!(6i64 / 3i64 == 2i64, 4100);
    assert!(9223372036854775807i64 / 7i64 == 1317624576693539401i64, 4101);
    assert!(-6i64 / 3i64 == -2i64, 4102);
    assert!(-9223372036854775807i64 / 7i64 == -1317624576693539401i64, 4103);
    assert!(6i64 / -3i64 == -2i64, 4104);
    assert!(9223372036854775807i64 / -7i64 == -1317624576693539401i64, 4105);
    assert!(-6i64 / -3i64 == 2i64, 4106);
    assert!(-9223372036854775807i64 / -7i64 == 1317624576693539401i64, 4107);
}
}

//# run --verbose
script {
fun main() {
    1i64 / 0i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -1i64 / 0i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -9223372036854775808i64 / -1i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    assert!(0i64 % 1i64 == 0i64, 5000);
    assert!(1i64 % 1i64 == 0i64, 5001);
    assert!(1i64 % 2i64 == 1i64, 5002);
    assert!(-1i64 % 1i64 == 0i64, 5003);
    assert!(-1i64 % 2i64 == -1i64, 5004);
    assert!(1i64 % -1i64 == 0i64, 5005);
    assert!(1i64 % -2i64 == 1i64, 5006);
    assert!(-1i64 % -1i64 == 0i64, 5007);
    assert!(-1i64 % -2i64 == -1i64, 5008);

    assert!(8i64 % 3i64 == 2i64, 5100);
    assert!(9223372036854775806i64 % 7i64 == 6i64, 5101);
    assert!(-8i64 % 3i64 == -2i64, 5102);
    assert!(-9223372036854775806i64 % 7i64 == -6i64, 5103);
    assert!(8i64 % -3i64 == 2i64, 5104);
    assert!(9223372036854775806i64 % -7i64 == 6i64, 5105);
    assert!(-8i64 % -3i64 == -2i64, 5106);
    assert!(-9223372036854775806i64 % -7i64 == -6i64, 5107);
}
}

//# run --verbose
script {
fun main() {
    1i64 % 0i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -1i64 % 0i64; // expect to abort
}
}

//# publish
module 0xff::negate_i64 {
  fun negate(a: i64): i64 {
    -a
  }
  public fun test1(){
    let a = 20i64;
    assert!(-a == negate(a));
  }
  public fun test2(){
    let a = -9223372036854775808i64;
    negate(a); // expect abort
  }
}

//# run 0xff::negate_i64::test1 --verbose

//# run 0xff::negate_i64::test2 --verbose
