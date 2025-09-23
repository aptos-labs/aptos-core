//# run --verbose
script {
fun main() {
    assert!(0i8 < 1i8, 1000);
    assert!(-1i8 < 0i8, 1001);
    assert!(0i8 <= 1i8, 1002);
    assert!(-1i8 <= 0i8, 1003);
    assert!(1i8 > 0i8, 1004);
    assert!(0i8 > -1i8, 1005);
    assert!(1i8 >= 0i8, 1006);
    assert!(0i8 >= -1i8, 1007);
}
}

//# run --verbose
script {
fun main() {
    assert!(0i16 < 1i16, 1000);
    assert!(-1i16 < 0i16, 1001);
    assert!(0i16 <= 1i16, 1002);
    assert!(-1i16 <= 0i16, 1003);
    assert!(1i16 > 0i16, 1004);
    assert!(0i16 > -1i16, 1005);
    assert!(1i16 >= 0i16, 1006);
    assert!(0i16 >= -1i16, 1007);
}
}

//# run --verbose
script {
fun main() {
    assert!(0i32 < 1i32, 1000);
    assert!(-1i32 < 0i32, 1001);
    assert!(0i32 <= 1i32, 1002);
    assert!(-1i32 <= 0i32, 1003);
    assert!(1i32 > 0i32, 1004);
    assert!(0i32 > -1i32, 1005);
    assert!(1i32 >= 0i32, 1006);
    assert!(0i32 >= -1i32, 1007);
}
}

//# run --verbose
script {
fun main() {
    assert!(0i64 < 1i64, 1000);
    assert!(-1i64 < 0i64, 1001);
    assert!(0i64 <= 1i64, 1002);
    assert!(-1i64 <= 0i64, 1003);
    assert!(1i64 > 0i64, 1004);
    assert!(0i64 > -1i64, 1005);
    assert!(1i64 >= 0i64, 1006);
    assert!(0i64 >= -1i64, 1007);
}
}

//# run --verbose
script {
fun main() {
    assert!(0i128 < 1i128, 1000);
    assert!(-1i128 < 0i128, 1001);
    assert!(0i128 <= 1i128, 1002);
    assert!(-1i128 <= 0i128, 1003);
    assert!(1i128 > 0i128, 1004);
    assert!(0i128 > -1i128, 1005);
    assert!(1i128 >= 0i128, 1006);
    assert!(0i128 >= -1i128, 1007);
}
}

//# run --verbose
script {
fun main() {
    assert!(0i256 < 1i256, 1000);
    assert!(-1i256 < 0i256, 1001);
    assert!(0i256 <= 1i256, 1002);
    assert!(-1i256 <= 0i256, 1003);
    assert!(1i256 > 0i256, 1004);
    assert!(0i256 > -1i256, 1005);
    assert!(1i256 >= 0i256, 1006);
    assert!(0i256 >= -1i256, 1007);
}
}
