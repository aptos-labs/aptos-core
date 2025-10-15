//# run --verbose
script {
  fun main() {
    assert!(0i8 + 0i8 == 0i8, 1000);
    assert!(0i8 + 1i8 == 1i8, 1001);
    assert!(1i8 + 1i8 == 2i8, 1002);
    assert!(0i8 + -1i8 == -1i8, 1003);
    assert!(-1i8 + -1i8 == -2i8, 1004);

    assert!(13i8 + 67i8 == 80i8, 1100);
    assert!(100i8 + 10i8 == 110i8, 1101);
    assert!(-13i8 + -67i8 == -80i8, 1102);
    assert!(-100i8 + -10i8 == -110i8, 1103);

    assert!(0i8 + 127i8 == 127i8, 1200);
    assert!(1i8 + 126i8 == 127i8, 1201);
    assert!(5i8 + 122i8 == 127i8, 1202);
    assert!(0i8 + -128i8 == -128i8, 1203);
    assert!(-1i8 + -127i8 == -128i8, 1204);
    assert!(-5i8 + -123i8 == -128i8, 1205);
  }
}

//# run --verbose
script {
  fun main() {
    1i8 + 127i8; // expect to abort
  }
}

//# run --verbose
script {
  fun main() {
    -1i8 + (-128i8); // expect to abort
  }
}

//# run --verbose
script {
fun main() {
    assert!(0i8 - 0i8 == 0i8, 2000);
    assert!(1i8 - 0i8 == 1i8, 2001);
    assert!(1i8 - 1i8 == 0i8, 2002);
    assert!(-1i8 - -0i8 == -1i8, 2003);
    assert!(-1i8 - -1i8 == 0i8, 2004);

    assert!(52i8 - 13i8 == 39i8, 2100);
    assert!(100i8 - 10i8 == 90i8, 2101);
    assert!(-52i8 - -13i8 == -39i8, 2102);
    assert!(-100i8 - -10i8 == -90i8, 2103);

    assert!(0i8 - -127i8 == 127i8, 2201);
    assert!(1i8 - -126i8 == 127i8, 2202);
    assert!(5i8 - -122i8 == 127i8, 2203);
    assert!(-1i8 - 127i8 == -128i8, 2204);
    assert!(-5i8 - 123i8 == -128i8, 2205);
    assert!(-128i8 - 0 == -128i8, 2206);
}
}

//# run --verbose
script {
  fun main() {
    -2i8 - 127i8; // expect to abort
  }
}

//# run --verbose
script {
  fun main() {
    1i8 - -127i8; // expect to abort
  }
}

//# run --verbose
script {
fun main() {
    assert!(0i8 * 0i8 == 0i8, 3000);
    assert!(1i8 * 0i8 == 0i8, 3001);
    assert!(1i8 * 1i8 == 1i8, 3002);
    assert!(-1i8 * 0i8 == 0i8, 3003);
    assert!(-1i8 * 1i8 == -1i8, 3004);
    assert!(1i8 * -1i8 == -1i8, 3005);
    assert!(-1i8 * -1i8 == 1i8, 3006);

    assert!(6i8 * 7i8 == 42i8, 3100);
    assert!(10i8 * 10i8 == 100i8, 3101);
    assert!(6i8 * -7i8 == -42i8, 3102);
    assert!(10i8 * -10i8 == -100i8, 3103);
    assert!(-6i8 * 7i8 == -42i8, 3104);
    assert!(-10i8 * 10i8 == -100i8, 3105);
    assert!(-6i8 * -7i8 == 42i8, 3106);
    assert!(-10i8 * -10i8 == 100i8, 3107);

    assert!(63i8 * 2i8 == 126i8, 3200);
    assert!(-63i8 * -2i8 == 126i8, 3201);
    assert!(-64i8 * 2i8 == -128i8, 3202);
    assert!(64i8 * -2i8 == -128i8, 3203);
}
}

//# run --verbose
script {
fun main() {
    8i8 * 16i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -8i8 * -16i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    9i8 * -16i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -9i8 * 16i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    assert!(0i8 / 1i8 == 0i8, 4000);
    assert!(1i8 / 1i8 == 1i8, 4001);
    assert!(1i8 / 2i8 == 0i8, 4002);
    assert!(0i8 / -1i8 == 0i8, 4003);
    assert!(1i8 / -1i8 == -1i8, 4004);
    assert!(1i8 / -2i8 == 0i8, 4005);
    assert!(-0i8 / 1i8 == 0i8, 4006);
    assert!(-1i8 / 1i8 == -1i8, 4007);
    assert!(-1i8 / 2i8 == 0i8, 4008);
    assert!(-0i8 / -1i8 == 0i8, 4009);
    assert!(-1i8 / -1i8 == 1i8, 40010);
    assert!(-1i8 / -2i8 == 0i8, 40011);

    assert!(6i8 / 3i8 == 2i8, 4100);
    assert!(127i8 / 7i8 == 18i8, 4101);
    assert!(-6i8 / 3i8 == -2i8, 4102);
    assert!(-127i8 / 7i8 == -18i8, 4103);
    assert!(6i8 / -3i8 == -2i8, 4104);
    assert!(127i8 / -7i8 == -18i8, 4105);
    assert!(-6i8 / -3i8 == 2i8, 4106);
    assert!(-127i8 / -7i8 == 18i8, 4107);
}
}

//# run --verbose
script {
fun main() {
    1i8 / 0i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -1i8 / 0i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -128i8 / -1i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    assert!(0i8 % 1i8 == 0i8, 5000);
    assert!(1i8 % 1i8 == 0i8, 5001);
    assert!(1i8 % 2i8 == 1i8, 5002);
    assert!(-0i8 % 1i8 == 0i8, 5003);
    assert!(-1i8 % 1i8 == 0i8, 5004);
    assert!(-1i8 % 2i8 == -1i8, 5005);
    assert!(0i8 % -1i8 == 0i8, 5006);
    assert!(1i8 % -1i8 == 0i8, 5007);
    assert!(1i8 % -2i8 == 1i8, 5008);
    assert!(-0i8 % -1i8 == 0i8, 5009);
    assert!(-1i8 % -1i8 == 0i8, 50010);
    assert!(-1i8 % -2i8 == -1i8, 50011);

    assert!(8i8 % 3i8 == 2i8, 5100);
    assert!(127i8 % 7i8 == 1i8, 5101);
    assert!(-8i8 % 3i8 == -2i8, 5102);
    assert!(-127i8 % 7i8 == -1i8, 5103);
    assert!(8i8 % -3i8 == 2i8, 5104);
    assert!(127i8 % -7i8 == 1i8, 5105);
    assert!(-8i8 % -3i8 == -2i8, 5106);
    assert!(-127i8 % -7i8 == -1i8, 5107);
}
}

//# run --verbose
script {
fun main() {
    1i8 % 0i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -1i8 % 0i8; // expect to abort
}
}

//# publish
module 0xff::negate_i8 {
  fun negate(a: i8): i8 {
    -a
  }
  public fun test1(){
    let a = 20i8;
    assert!(-a == negate(a));
  }
  public fun test2(){
    let a = -128i8;
    negate(a); // expect abort
  }
}

//# run 0xff::negate_i8::test1 --verbose

//# run 0xff::negate_i8::test2 --verbose
