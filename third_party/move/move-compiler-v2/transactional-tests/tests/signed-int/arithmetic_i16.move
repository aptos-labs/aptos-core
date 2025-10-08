//# run --verbose
script {
  fun main() {
    assert!(0i16 + 0i16 == 0i16, 1000);
    assert!(0i16 + 1i16 == 1i16, 1001);
    assert!(1i16 + 1i16 == 2i16, 1002);
    assert!(0i16 + -1i16 == -1i16, 1003);
    assert!(-1i16 + -1i16 == -2i16, 1004);

    assert!(13i16 + 67i16 == 80i16, 1100);
    assert!(100i16 + 10i16 == 110i16, 1101);
    assert!(-13i16 + -67i16 == -80i16, 1102);
    assert!(-100i16 + -10i16 == -110i16, 1103);

    assert!(0i16 + 32767i16 == 32767i16, 1200);
    assert!(1i16 + 32766i16 == 32767i16, 1201);
    assert!(5i16 + 32762i16 == 32767i16, 1202);
    assert!(0i16 + -32768i16 == -32768i16, 1203);
    assert!(-1i16 + -32767i16 == -32768i16, 1204);
    assert!(-5i16 + -32763i16 == -32768i16, 1205);
  }
}

//# run --verbose
script {
  fun main() {
    1i16 + 32767i16; // expect to abort
  }
}

//# run --verbose
script {
  fun main() {
    -1i16 + (-32768i16); // expect to abort
  }
}

//# run --verbose
script {
fun main() {
    assert!(0i16 - 0i16 == 0i16, 2000);
    assert!(1i16 - 0i16 == 1i16, 2001);
    assert!(1i16 - 1i16 == 0i16, 2002);
    assert!(-1i16 - -0i16 == -1i16, 2003);
    assert!(-1i16 - -1i16 == 0i16, 2004);

    assert!(52i16 - 13i16 == 39i16, 2100);
    assert!(100i16 - 10i16 == 90i16, 2101);
    assert!(-52i16 - -13i16 == -39i16, 2102);
    assert!(-100i16 - -10i16 == -90i16, 2103);

    assert!(0i16 - -32767i16 == 32767i16, 2201);
    assert!(1i16 - -32766i16 == 32767i16, 2201);
    assert!(5i16 - -32762i16 == 32767i16, 2201);
    assert!(-1i16 - 32767i16 == -32768i16, 2201);
    assert!(-5i16 - 32763i16 == -32768i16, 2201);
    assert!(-32768i16 - 0 == -32768i16, 2201);
}
}

//# run --verbose
script {
  fun main() {
    -2i16 - 32767i16; // expect to abort
  }
}

//# run --verbose
script {
  fun main() {
    1i16 - -32767i16; // expect to abort
  }
}

//# run --verbose
script {
fun main() {
    assert!(0i16 * 0i16 == 0i16, 3000);
    assert!(1i16 * 0i16 == 0i16, 3001);
    assert!(1i16 * 1i16 == 1i16, 3002);
    assert!(-1i16 * 0i16 == 0i16, 3003);
    assert!(-1i16 * 1i16 == -1i16, 3004);
    assert!(1i16 * -1i16 == -1i16, 3005);
    assert!(-1i16 * -1i16 == 1i16, 3006);

    assert!(6i16 * 7i16 == 42i16, 3100);
    assert!(10i16 * 10i16 == 100i16, 3101);
    assert!(6i16 * -7i16 == -42i16, 3102);
    assert!(10i16 * -10i16 == -100i16, 3103);
    assert!(-6i16 * 7i16 == -42i16, 3104);
    assert!(-10i16 * 10i16 == -100i16, 3105);
    assert!(-6i16 * -7i16 == 42i16, 3106);
    assert!(-10i16 * -10i16 == 100i16, 3107);

    assert!(127i16 * 256i16 == 32512i16, 3200);
    assert!(-127i16 * -256i16 == 32512i16, 3201);
    assert!(-127i16 * 256i16 == -32512i16, 3202);
    assert!(127i16 * -256i16 == -32512i16, 3203);
}
}

//# run --verbose
script {
fun main() {
    128i16 * 256i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -128i16 * -256i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    129i16 * -256i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -129i16 * 256i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    assert!(0i16 / 1i16 == 0i16, 4000);
    assert!(1i16 / 1i16 == 1i16, 4001);
    assert!(1i16 / 2i16 == 0i16, 4002);
    assert!(0i16 / -1i16 == 0i16, 4003);
    assert!(1i16 / -1i16 == -1i16, 4004);
    assert!(1i16 / -2i16 == 0i16, 4005);
    assert!(-0i16 / 1i16 == 0i16, 4006);
    assert!(-1i16 / 1i16 == -1i16, 4007);
    assert!(-1i16 / 2i16 == 0i16, 4008);
    assert!(-0i16 / -1i16 == 0i16, 4009);
    assert!(-1i16 / -1i16 == 1i16, 40010);
    assert!(-1i16 / -2i16 == 0i16, 40011);

    assert!(6i16 / 3i16 == 2i16, 4100);
    assert!(32767i16 / 7i16 == 4681i16, 4101);
    assert!(-6i16 / 3i16 == -2i16, 4102);
    assert!(-32767i16 / 7i16 == -4681i16, 4103);
    assert!(6i16 / -3i16 == -2i16, 4104);
    assert!(32767i16 / -7i16 == -4681i16, 4105);
    assert!(-6i16 / -3i16 == 2i16, 4106);
    assert!(-32767i16 / -7i16 == 4681i16, 4107);
}
}

//# run --verbose
script {
fun main() {
    1i16 / 0i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -1i16 / 0i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -32768i16 / -1i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    assert!(0i16 % 1i16 == 0i16, 5000);
    assert!(1i16 % 1i16 == 0i16, 5001);
    assert!(1i16 % 2i16 == 1i16, 5002);
    assert!(-0i16 % 1i16 == 0i16, 5003);
    assert!(-1i16 % 1i16 == 0i16, 5004);
    assert!(-1i16 % 2i16 == -1i16, 5005);
    assert!(0i16 % -1i16 == 0i16, 5006);
    assert!(1i16 % -1i16 == 0i16, 5007);
    assert!(1i16 % -2i16 == 1i16, 5008);
    assert!(-0i16 % -1i16 == 0i16, 5009);
    assert!(-1i16 % -1i16 == 0i16, 50010);
    assert!(-1i16 % -2i16 == -1i16, 50011);

    assert!(8i16 % 3i16 == 2i16, 5100);
    assert!(32765i16 % 7i16 == 5i16, 5101);
    assert!(-8i16 % 3i16 == -2i16, 5102);
    assert!(-32765i16 % 7i16 == -5i16, 5103);
    assert!(8i16 % -3i16 == 2i16, 5104);
    assert!(32765i16 % -7i16 == 5i16, 5105);
    assert!(-8i16 % -3i16 == -2i16, 5106);
    assert!(-32765i16 % -7i16 == -5i16, 5107);
}
}

//# run --verbose
script {
fun main() {
    1i16 % 0i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -1i16 % 0i16; // expect to abort
}
}

//# publish
module 0xff::negate_i16 {
  fun negate(a: i16): i16 {
    -a
  }
  public fun test1(){
    let a = 20i16;
    assert!(-a == negate(a));
  }
  public fun test2(){
    let a = -32768i16;
    negate(a); // expect abort
  }
}

//# run 0xff::negate_i16::test1 --verbose

//# run 0xff::negate_i16::test2 --verbose
