; ModuleID = '0x100__Test'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

declare i32 @memcmp(ptr, ptr, i64)

define private i128 @Test__takes_u128(i128 %0) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i128, align 8
  store i128 %0, ptr %local_0, align 8
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_1, align 8
  %retval = load i128, ptr %local_1, align 8
  ret i128 %retval
}

define private i128 @Test__test_const_u128() {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i128, align 8
  %local_2 = alloca i128, align 8
  %local_3 = alloca i128, align 8
  %local_4 = alloca i128, align 8
  %local_5 = alloca i128, align 8
  %local_6 = alloca i128, align 8
  %local_7 = alloca i128, align 8
  store i128 7, ptr %local_0, align 8
  %call_arg_0 = load i128, ptr %local_0, align 8
  %retval = call i128 @Test__takes_u128(i128 %call_arg_0)
  store i128 %retval, ptr %local_1, align 8
  store i128 4294967296, ptr %local_2, align 8
  %call_arg_01 = load i128, ptr %local_2, align 8
  %retval2 = call i128 @Test__takes_u128(i128 %call_arg_01)
  store i128 %retval2, ptr %local_3, align 8
  store i128 18446744073709551616, ptr %local_4, align 8
  %call_arg_03 = load i128, ptr %local_4, align 8
  %retval4 = call i128 @Test__takes_u128(i128 %call_arg_03)
  store i128 %retval4, ptr %local_5, align 8
  store i128 -170141183460469231731687303715884105728, ptr %local_6, align 8
  %call_arg_05 = load i128, ptr %local_6, align 8
  %retval6 = call i128 @Test__takes_u128(i128 %call_arg_05)
  store i128 %retval6, ptr %local_7, align 8
  %retval7 = load i128, ptr %local_7, align 8
  ret i128 %retval7
}
