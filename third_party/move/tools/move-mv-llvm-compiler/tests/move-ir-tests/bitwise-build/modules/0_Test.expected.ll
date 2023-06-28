; ModuleID = '0x100__Test'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

declare i32 @memcmp(ptr, ptr, i64)

define private i8 @Test__test_and(i8 %0, i8 %1) {
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
  %and_src_0 = load i8, ptr %local_2, align 1
  %and_src_1 = load i8, ptr %local_3, align 1
  %and_dst = and i8 %and_src_0, %and_src_1
  store i8 %and_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

define private i8 @Test__test_or(i8 %0, i8 %1) {
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
  %or_src_0 = load i8, ptr %local_2, align 1
  %or_src_1 = load i8, ptr %local_3, align 1
  %or_dst = or i8 %or_src_0, %or_src_1
  store i8 %or_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

define private i128 @Test__test_shl128(i128 %0, i8 %1) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i8, align 1
  %local_2 = alloca i128, align 8
  %local_3 = alloca i8, align 1
  %local_4 = alloca i128, align 8
  store i128 %0, ptr %local_0, align 8
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %shl_src_0 = load i128, ptr %local_2, align 8
  %shl_src_1 = load i8, ptr %local_3, align 1
  %rangecond = icmp uge i8 %shl_src_1, -128
  br i1 %rangecond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %zext_dst = zext i8 %shl_src_1 to i128
  %shl_dst = shl i128 %shl_src_0, %zext_dst
  store i128 %shl_dst, ptr %local_4, align 8
  %retval = load i128, ptr %local_4, align 8
  ret i128 %retval
}

define private i32 @Test__test_shl32(i32 %0, i8 %1) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i8, align 1
  %local_2 = alloca i32, align 4
  %local_3 = alloca i8, align 1
  %local_4 = alloca i32, align 4
  store i32 %0, ptr %local_0, align 4
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_2, align 4
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %shl_src_0 = load i32, ptr %local_2, align 4
  %shl_src_1 = load i8, ptr %local_3, align 1
  %rangecond = icmp uge i8 %shl_src_1, 32
  br i1 %rangecond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %zext_dst = zext i8 %shl_src_1 to i32
  %shl_dst = shl i32 %shl_src_0, %zext_dst
  store i32 %shl_dst, ptr %local_4, align 4
  %retval = load i32, ptr %local_4, align 4
  ret i32 %retval
}

define private i64 @Test__test_shl64(i64 %0, i8 %1) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i8, align 1
  %local_2 = alloca i64, align 8
  %local_3 = alloca i8, align 1
  %local_4 = alloca i64, align 8
  store i64 %0, ptr %local_0, align 8
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %shl_src_0 = load i64, ptr %local_2, align 8
  %shl_src_1 = load i8, ptr %local_3, align 1
  %rangecond = icmp uge i8 %shl_src_1, 64
  br i1 %rangecond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %zext_dst = zext i8 %shl_src_1 to i64
  %shl_dst = shl i64 %shl_src_0, %zext_dst
  store i64 %shl_dst, ptr %local_4, align 8
  %retval = load i64, ptr %local_4, align 8
  ret i64 %retval
}

define private i8 @Test__test_shl8(i8 %0, i8 %1) {
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
  %shl_src_0 = load i8, ptr %local_2, align 1
  %shl_src_1 = load i8, ptr %local_3, align 1
  %rangecond = icmp uge i8 %shl_src_1, 8
  br i1 %rangecond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %shl_dst = shl i8 %shl_src_0, %shl_src_1
  store i8 %shl_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

define private i128 @Test__test_shr128(i128 %0, i8 %1) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i8, align 1
  %local_2 = alloca i128, align 8
  %local_3 = alloca i8, align 1
  %local_4 = alloca i128, align 8
  store i128 %0, ptr %local_0, align 8
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %shr_src_0 = load i128, ptr %local_2, align 8
  %shr_src_1 = load i8, ptr %local_3, align 1
  %rangecond = icmp uge i8 %shr_src_1, -128
  br i1 %rangecond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %zext_dst = zext i8 %shr_src_1 to i128
  %shr_dst = lshr i128 %shr_src_0, %zext_dst
  store i128 %shr_dst, ptr %local_4, align 8
  %retval = load i128, ptr %local_4, align 8
  ret i128 %retval
}

define private i32 @Test__test_shr32(i32 %0, i8 %1) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i8, align 1
  %local_2 = alloca i32, align 4
  %local_3 = alloca i8, align 1
  %local_4 = alloca i32, align 4
  store i32 %0, ptr %local_0, align 4
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_2, align 4
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %shr_src_0 = load i32, ptr %local_2, align 4
  %shr_src_1 = load i8, ptr %local_3, align 1
  %rangecond = icmp uge i8 %shr_src_1, 32
  br i1 %rangecond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %zext_dst = zext i8 %shr_src_1 to i32
  %shr_dst = lshr i32 %shr_src_0, %zext_dst
  store i32 %shr_dst, ptr %local_4, align 4
  %retval = load i32, ptr %local_4, align 4
  ret i32 %retval
}

define private i64 @Test__test_shr64(i64 %0, i8 %1) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i8, align 1
  %local_2 = alloca i64, align 8
  %local_3 = alloca i8, align 1
  %local_4 = alloca i64, align 8
  store i64 %0, ptr %local_0, align 8
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %shr_src_0 = load i64, ptr %local_2, align 8
  %shr_src_1 = load i8, ptr %local_3, align 1
  %rangecond = icmp uge i8 %shr_src_1, 64
  br i1 %rangecond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %zext_dst = zext i8 %shr_src_1 to i64
  %shr_dst = lshr i64 %shr_src_0, %zext_dst
  store i64 %shr_dst, ptr %local_4, align 8
  %retval = load i64, ptr %local_4, align 8
  ret i64 %retval
}

define private i8 @Test__test_shr8(i8 %0, i8 %1) {
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
  %shr_src_0 = load i8, ptr %local_2, align 1
  %shr_src_1 = load i8, ptr %local_3, align 1
  %rangecond = icmp uge i8 %shr_src_1, 8
  br i1 %rangecond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %shr_dst = lshr i8 %shr_src_0, %shr_src_1
  store i8 %shr_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

define private i8 @Test__test_xor(i8 %0, i8 %1) {
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
  %xor_src_0 = load i8, ptr %local_2, align 1
  %xor_src_1 = load i8, ptr %local_3, align 1
  %xor_dst = xor i8 %xor_src_0, %xor_src_1
  store i8 %xor_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

; Function Attrs: noreturn
declare void @move_rt_abort(i64) #0

attributes #0 = { noreturn }
