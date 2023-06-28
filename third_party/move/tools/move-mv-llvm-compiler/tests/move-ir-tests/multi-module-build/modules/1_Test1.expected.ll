; ModuleID = '0x100__Test1'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

declare i32 @memcmp(ptr, ptr, i64)

define i8 @Test1__test1(i8 %0, i8 %1) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i8, align 1
  store i8 %0, ptr %local_0, align 1
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %call_arg_0 = load i8, ptr %local_2, align 1
  %call_arg_1 = load i8, ptr %local_3, align 1
  %retval = call i8 @Test2__test2(i8 %call_arg_0, i8 %call_arg_1)
  store i8 %retval, ptr %local_4, align 1
  %retval2 = load i8, ptr %local_4, align 1
  ret i8 %retval2
}

declare i8 @Test2__test2(i8, i8)
