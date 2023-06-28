; ModuleID = '0x100__M3'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

@acct.addr = internal constant [32 x i8] c"\1F\1E\1D\1C\1B\1A\19\18\17\16\15\14\13\12\11\10\0F\0E\0D\0C\0B\0A\09\08\07\06\05\04\03\02\01\00"

declare i32 @memcmp(ptr, ptr, i64)

define i1 @M3__eq_address([32 x i8] %0, [32 x i8] %1) {
entry:
  %local_0 = alloca [32 x i8], align 1
  %local_1 = alloca [32 x i8], align 1
  %local_2 = alloca [32 x i8], align 1
  %local_3 = alloca [32 x i8], align 1
  %local_4 = alloca i1, align 1
  store [32 x i8] %0, ptr %local_0, align 1
  store [32 x i8] %1, ptr %local_1, align 1
  %2 = call i32 @memcmp(ptr %local_0, ptr %local_1, i64 32)
  %eq_dst = icmp eq i32 %2, 0
  store i1 %eq_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define [32 x i8] @M3__fixed_address() {
entry:
  %local_0 = alloca [32 x i8], align 1
  %0 = load [32 x i8], ptr @acct.addr, align 1
  store [32 x i8] %0, ptr %local_0, align 1
  %retval = load [32 x i8], ptr %local_0, align 1
  ret [32 x i8] %retval
}

define i1 @M3__ne_address([32 x i8] %0, [32 x i8] %1) {
entry:
  %local_0 = alloca [32 x i8], align 1
  %local_1 = alloca [32 x i8], align 1
  %local_2 = alloca [32 x i8], align 1
  %local_3 = alloca [32 x i8], align 1
  %local_4 = alloca i1, align 1
  store [32 x i8] %0, ptr %local_0, align 1
  store [32 x i8] %1, ptr %local_1, align 1
  %2 = call i32 @memcmp(ptr %local_0, ptr %local_1, i64 32)
  %ne_dst = icmp ne i32 %2, 0
  store i1 %ne_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define ptr @M3__ret_address_ref(ptr %0) {
entry:
  %local_0 = alloca ptr, align 8
  %local_1 = alloca ptr, align 8
  store ptr %0, ptr %local_0, align 8
  %load_store_tmp = load ptr, ptr %local_0, align 8
  store ptr %load_store_tmp, ptr %local_1, align 8
  %retval = load ptr, ptr %local_1, align 8
  ret ptr %retval
}

define [32 x i8] @M3__use_address_ref(ptr %0) {
entry:
  %local_0 = alloca ptr, align 8
  %local_1 = alloca ptr, align 8
  %local_2 = alloca [32 x i8], align 1
  store ptr %0, ptr %local_0, align 8
  %load_store_tmp = load ptr, ptr %local_0, align 8
  store ptr %load_store_tmp, ptr %local_1, align 8
  %load_deref_store_tmp1 = load ptr, ptr %local_1, align 8
  %load_deref_store_tmp2 = load [32 x i8], ptr %load_deref_store_tmp1, align 1
  store [32 x i8] %load_deref_store_tmp2, ptr %local_2, align 1
  %retval = load [32 x i8], ptr %local_2, align 1
  ret [32 x i8] %retval
}

define [32 x i8] @M3__use_address_val([32 x i8] %0) {
entry:
  %local_0 = alloca [32 x i8], align 1
  %local_1 = alloca [32 x i8], align 1
  store [32 x i8] %0, ptr %local_0, align 1
  %retval = load [32 x i8], ptr %local_0, align 1
  ret [32 x i8] %retval
}
