//# run --verbose
script {
  fun main() {
    assert!(0i32 + 0i32 == 0i32, 1000);
    assert!(0i32 + 1i32 == 1i32, 1001);
    assert!(1i32 + 1i32 == 2i32, 1002);
    assert!(0i32 + -1i32 == -1i32, 1003);
    assert!(-1i32 + -1i32 == -2i32, 1004);

    assert!(13i32 + 67i32 == 80i32, 1100);
    assert!(100i32 + 10i32 == 110i32, 1101);
    assert!(-13i32 + -67i32 == -80i32, 1102);
    assert!(-100i32 + -10i32 == -110i32, 1103);

    assert!(0i32 + 2147483647i32 == 2147483647i32, 1200);
    assert!(1i32 + 2147483646i32 == 2147483647i32, 1201);
    assert!(5i32 + 2147483642i32 == 2147483647i32, 1202);
    assert!(0i32 + -2147483648i32 == -2147483648i32, 1203);
    assert!(-1i32 + -2147483647i32 == -2147483648i32, 1204);
    assert!(-5i32 + -2147483643i32 == -2147483648i32, 1205);
  }
}

//# run --verbose
script {
  fun main() {
    1i32 + 2147483647i32; // expect to abort
  }
}

//# run --verbose
script {
  fun main() {
    -1i32 + (-2147483648i32); // expect to abort
  }
}

//# run --verbose
script {
fun main() {
    assert!(0i32 - 0i32 == 0i32, 2000);
    assert!(1i32 - 0i32 == 1i32, 2001);
    assert!(1i32 - 1i32 == 0i32, 2002);
    assert!(-1i32 - -0i32 == -1i32, 2003);
    assert!(-1i32 - -1i32 == 0i32, 2004);

    assert!(52i32 - 13i32 == 39i32, 2100);
    assert!(100i32 - 10i32 == 90i32, 2101);
    assert!(-52i32 - -13i32 == -39i32, 2102);
    assert!(-100i32 - -10i32 == -90i32, 2103);

    assert!(0i32 - -2147483647i32 == 2147483647i32, 2200);
    assert!(1i32 - -2147483646i32 == 2147483647i32, 2201);
    assert!(5i32 - -2147483642i32 == 2147483647i32, 2202);
    assert!(-1i32 - 2147483647i32 == -2147483648i32, 2203);
    assert!(-5i32 - 2147483643i32 == -2147483648i32, 2204);
    assert!(-2147483648i32 - 0 == -2147483648i32, 2205);
}
}

//# run --verbose
script {
  fun main() {
    -2i32 - 2147483647i32; // expect to abort
  }
}

//# run --verbose
script {
  fun main() {
    1i32 - -2147483647i32; // expect to abort
  }
}

//# run --verbose
script {
fun main() {
    assert!(0i32 * 0i32 == 0i32, 3000);
    assert!(1i32 * 0i32 == 0i32, 3001);
    assert!(1i32 * 1i32 == 1i32, 3002);
    assert!(-1i32 * 0i32 == 0i32, 3003);
    assert!(-1i32 * 1i32 == -1i32, 3004);
    assert!(1i32 * -1i32 == -1i32, 3005);
    assert!(-1i32 * -1i32 == 1i32, 3006);

    assert!(6i32 * 7i32 == 42i32, 3100);
    assert!(10i32 * 10i32 == 100i32, 3101);
    assert!(6i32 * -7i32 == -42i32, 3102);
    assert!(10i32 * -10i32 == -100i32, 3103);
    assert!(-6i32 * 7i32 == -42i32, 3104);
    assert!(-10i32 * 10i32 == -100i32, 3105);
    assert!(-6i32 * -7i32 == 42i32, 3106);
    assert!(-10i32 * -10i32 == 100i32, 3107);

    assert!(46340i32 * 46340i32 == 2147395600i32, 3200);
    assert!(-46340i32 * -46340i32 == 2147395600i32, 3201);
    assert!(46340i32 * -46340i32 == -2147395600i32, 3202);
    assert!(-46340i32 * 46340i32 == -2147395600i32, 3203);
}
}

//# run --verbose
script {
fun main() {
    46341i32 * 46341i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -46341i32 * -46341i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    2147483647i32 * 2i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -2147483648i32 * -1i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    assert!(0i32 / 1i32 == 0i32, 4000);
    assert!(1i32 / 1i32 == 1i32, 4001);
    assert!(1i32 / 2i32 == 0i32, 4002);
    assert!(0i32 / -1i32 == 0i32, 4003);
    assert!(1i32 / -1i32 == -1i32, 4004);
    assert!(1i32 / -2i32 == 0i32, 4005);
    assert!(-1i32 / 1i32 == -1i32, 4006);
    assert!(-1i32 / 2i32 == 0i32, 4007);
    assert!(-1i32 / -1i32 == 1i32, 4008);
    assert!(-1i32 / -2i32 == 0i32, 4009);

    assert!(6i32 / 3i32 == 2i32, 4100);
    assert!(2147483647i32 / 7i32 == 306783378i32, 4101);
    assert!(-6i32 / 3i32 == -2i32, 4102);
    assert!(-2147483647i32 / 7i32 == -306783378i32, 4103);
    assert!(6i32 / -3i32 == -2i32, 4104);
    assert!(2147483647i32 / -7i32 == -306783378i32, 4105);
    assert!(-6i32 / -3i32 == 2i32, 4106);
    assert!(-2147483647i32 / -7i32 == 306783378i32, 4107);
}
}

//# run --verbose
script {
fun main() {
    1i32 / 0i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -1i32 / 0i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -2147483648i32 / -1i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    assert!(0i32 % 1i32 == 0i32, 5000);
    assert!(1i32 % 1i32 == 0i32, 5001);
    assert!(1i32 % 2i32 == 1i32, 5002);
    assert!(-1i32 % 1i32 == 0i32, 5003);
    assert!(-1i32 % 2i32 == -1i32, 5004);
    assert!(1i32 % -1i32 == 0i32, 5005);
    assert!(1i32 % -2i32 == 1i32, 5006);
    assert!(-1i32 % -1i32 == 0i32, 5007);
    assert!(-1i32 % -2i32 == -1i32, 5008);

    assert!(8i32 % 3i32 == 2i32, 5100);
    assert!(2147483647i32 % 7i32 == 1i32, 5101);
    assert!(-8i32 % 3i32 == -2i32, 5102);
    assert!(-2147483647i32 % 7i32 == -1i32, 5103);
    assert!(8i32 % -3i32 == 2i32, 5104);
    assert!(2147483647i32 % -7i32 == 1i32, 5105);
    assert!(-8i32 % -3i32 == -2i32, 5106);
    assert!(-2147483647i32 % -7i32 == -1i32, 5107);
}
}

//# run --verbose
script {
fun main() {
    1i32 % 0i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
    -1i32 % 0i32; // expect to abort
}
}


//# publish
module 0xff::negate_i32 {
  fun negate(a: i32): i32 {
    -a
  }
  public fun test1(){
    let a = 20i32;
    assert!(-a == negate(a));
  }
  public fun test2(){
    let a = -2147483648i32;
    negate(a); // expect abort
  }
}

//# run 0xff::negate_i32::test1 --verbose

//# run 0xff::negate_i32::test2 --verbose
