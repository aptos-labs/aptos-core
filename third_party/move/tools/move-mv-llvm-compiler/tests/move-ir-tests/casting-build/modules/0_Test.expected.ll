; ModuleID = '0x100__Test'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

declare i32 @memcmp(ptr, ptr, i64)

define private i128 @Test__cast_u128_as_u128(i128 %0) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i128, align 8
  %local_2 = alloca i128, align 8
  store i128 %0, ptr %local_0, align 8
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i128, ptr %local_1, align 8
  store i128 %cast_src, ptr %local_2, align 8
  %retval = load i128, ptr %local_2, align 8
  ret i128 %retval
}

define private i16 @Test__cast_u128_as_u16(i128 %0) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i128, align 8
  %local_2 = alloca i16, align 2
  store i128 %0, ptr %local_0, align 8
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i128, ptr %local_1, align 8
  %castcond = icmp ugt i128 %cast_src, 65535
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i128 %cast_src to i16
  store i16 %trunc_dst, ptr %local_2, align 2
  %retval = load i16, ptr %local_2, align 2
  ret i16 %retval
}

define private i256 @Test__cast_u128_as_u256(i128 %0) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i128, align 8
  %local_2 = alloca i256, align 8
  store i128 %0, ptr %local_0, align 8
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i128, ptr %local_1, align 8
  %zext_dst = zext i128 %cast_src to i256
  store i256 %zext_dst, ptr %local_2, align 8
  %retval = load i256, ptr %local_2, align 8
  ret i256 %retval
}

define private i32 @Test__cast_u128_as_u32(i128 %0) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i128, align 8
  %local_2 = alloca i32, align 4
  store i128 %0, ptr %local_0, align 8
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i128, ptr %local_1, align 8
  %castcond = icmp ugt i128 %cast_src, 4294967295
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i128 %cast_src to i32
  store i32 %trunc_dst, ptr %local_2, align 4
  %retval = load i32, ptr %local_2, align 4
  ret i32 %retval
}

define private i64 @Test__cast_u128_as_u64(i128 %0) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i128, align 8
  %local_2 = alloca i64, align 8
  store i128 %0, ptr %local_0, align 8
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i128, ptr %local_1, align 8
  %castcond = icmp ugt i128 %cast_src, 18446744073709551615
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i128 %cast_src to i64
  store i64 %trunc_dst, ptr %local_2, align 8
  %retval = load i64, ptr %local_2, align 8
  ret i64 %retval
}

define private i8 @Test__cast_u128_as_u8(i128 %0) {
entry:
  %local_0 = alloca i128, align 8
  %local_1 = alloca i128, align 8
  %local_2 = alloca i8, align 1
  store i128 %0, ptr %local_0, align 8
  %load_store_tmp = load i128, ptr %local_0, align 8
  store i128 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i128, ptr %local_1, align 8
  %castcond = icmp ugt i128 %cast_src, 255
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i128 %cast_src to i8
  store i8 %trunc_dst, ptr %local_2, align 1
  %retval = load i8, ptr %local_2, align 1
  ret i8 %retval
}

define private i128 @Test__cast_u16_as_u128(i16 %0) {
entry:
  %local_0 = alloca i16, align 2
  %local_1 = alloca i16, align 2
  %local_2 = alloca i128, align 8
  store i16 %0, ptr %local_0, align 2
  %load_store_tmp = load i16, ptr %local_0, align 2
  store i16 %load_store_tmp, ptr %local_1, align 2
  %cast_src = load i16, ptr %local_1, align 2
  %zext_dst = zext i16 %cast_src to i128
  store i128 %zext_dst, ptr %local_2, align 8
  %retval = load i128, ptr %local_2, align 8
  ret i128 %retval
}

define private i16 @Test__cast_u16_as_u16(i16 %0) {
entry:
  %local_0 = alloca i16, align 2
  %local_1 = alloca i16, align 2
  %local_2 = alloca i16, align 2
  store i16 %0, ptr %local_0, align 2
  %load_store_tmp = load i16, ptr %local_0, align 2
  store i16 %load_store_tmp, ptr %local_1, align 2
  %cast_src = load i16, ptr %local_1, align 2
  store i16 %cast_src, ptr %local_2, align 2
  %retval = load i16, ptr %local_2, align 2
  ret i16 %retval
}

define private i256 @Test__cast_u16_as_u256(i16 %0) {
entry:
  %local_0 = alloca i16, align 2
  %local_1 = alloca i16, align 2
  %local_2 = alloca i256, align 8
  store i16 %0, ptr %local_0, align 2
  %load_store_tmp = load i16, ptr %local_0, align 2
  store i16 %load_store_tmp, ptr %local_1, align 2
  %cast_src = load i16, ptr %local_1, align 2
  %zext_dst = zext i16 %cast_src to i256
  store i256 %zext_dst, ptr %local_2, align 8
  %retval = load i256, ptr %local_2, align 8
  ret i256 %retval
}

define private i32 @Test__cast_u16_as_u32(i16 %0) {
entry:
  %local_0 = alloca i16, align 2
  %local_1 = alloca i16, align 2
  %local_2 = alloca i32, align 4
  store i16 %0, ptr %local_0, align 2
  %load_store_tmp = load i16, ptr %local_0, align 2
  store i16 %load_store_tmp, ptr %local_1, align 2
  %cast_src = load i16, ptr %local_1, align 2
  %zext_dst = zext i16 %cast_src to i32
  store i32 %zext_dst, ptr %local_2, align 4
  %retval = load i32, ptr %local_2, align 4
  ret i32 %retval
}

define private i64 @Test__cast_u16_as_u64(i16 %0) {
entry:
  %local_0 = alloca i16, align 2
  %local_1 = alloca i16, align 2
  %local_2 = alloca i64, align 8
  store i16 %0, ptr %local_0, align 2
  %load_store_tmp = load i16, ptr %local_0, align 2
  store i16 %load_store_tmp, ptr %local_1, align 2
  %cast_src = load i16, ptr %local_1, align 2
  %zext_dst = zext i16 %cast_src to i64
  store i64 %zext_dst, ptr %local_2, align 8
  %retval = load i64, ptr %local_2, align 8
  ret i64 %retval
}

define private i8 @Test__cast_u16_as_u8(i16 %0) {
entry:
  %local_0 = alloca i16, align 2
  %local_1 = alloca i16, align 2
  %local_2 = alloca i8, align 1
  store i16 %0, ptr %local_0, align 2
  %load_store_tmp = load i16, ptr %local_0, align 2
  store i16 %load_store_tmp, ptr %local_1, align 2
  %cast_src = load i16, ptr %local_1, align 2
  %castcond = icmp ugt i16 %cast_src, 255
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i16 %cast_src to i8
  store i8 %trunc_dst, ptr %local_2, align 1
  %retval = load i8, ptr %local_2, align 1
  ret i8 %retval
}

define private i128 @Test__cast_u256_as_u128(i256 %0) {
entry:
  %local_0 = alloca i256, align 8
  %local_1 = alloca i256, align 8
  %local_2 = alloca i128, align 8
  store i256 %0, ptr %local_0, align 8
  %load_store_tmp = load i256, ptr %local_0, align 8
  store i256 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i256, ptr %local_1, align 8
  %castcond = icmp ugt i256 %cast_src, 340282366920938463463374607431768211455
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i256 %cast_src to i128
  store i128 %trunc_dst, ptr %local_2, align 8
  %retval = load i128, ptr %local_2, align 8
  ret i128 %retval
}

define private i16 @Test__cast_u256_as_u16(i256 %0) {
entry:
  %local_0 = alloca i256, align 8
  %local_1 = alloca i256, align 8
  %local_2 = alloca i16, align 2
  store i256 %0, ptr %local_0, align 8
  %load_store_tmp = load i256, ptr %local_0, align 8
  store i256 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i256, ptr %local_1, align 8
  %castcond = icmp ugt i256 %cast_src, 65535
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i256 %cast_src to i16
  store i16 %trunc_dst, ptr %local_2, align 2
  %retval = load i16, ptr %local_2, align 2
  ret i16 %retval
}

define private i256 @Test__cast_u256_as_u256(i256 %0) {
entry:
  %local_0 = alloca i256, align 8
  %local_1 = alloca i256, align 8
  %local_2 = alloca i256, align 8
  store i256 %0, ptr %local_0, align 8
  %load_store_tmp = load i256, ptr %local_0, align 8
  store i256 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i256, ptr %local_1, align 8
  store i256 %cast_src, ptr %local_2, align 8
  %retval = load i256, ptr %local_2, align 8
  ret i256 %retval
}

define private i32 @Test__cast_u256_as_u32(i256 %0) {
entry:
  %local_0 = alloca i256, align 8
  %local_1 = alloca i256, align 8
  %local_2 = alloca i32, align 4
  store i256 %0, ptr %local_0, align 8
  %load_store_tmp = load i256, ptr %local_0, align 8
  store i256 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i256, ptr %local_1, align 8
  %castcond = icmp ugt i256 %cast_src, 4294967295
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i256 %cast_src to i32
  store i32 %trunc_dst, ptr %local_2, align 4
  %retval = load i32, ptr %local_2, align 4
  ret i32 %retval
}

define private i64 @Test__cast_u256_as_u64(i256 %0) {
entry:
  %local_0 = alloca i256, align 8
  %local_1 = alloca i256, align 8
  %local_2 = alloca i64, align 8
  store i256 %0, ptr %local_0, align 8
  %load_store_tmp = load i256, ptr %local_0, align 8
  store i256 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i256, ptr %local_1, align 8
  %castcond = icmp ugt i256 %cast_src, 18446744073709551615
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i256 %cast_src to i64
  store i64 %trunc_dst, ptr %local_2, align 8
  %retval = load i64, ptr %local_2, align 8
  ret i64 %retval
}

define private i8 @Test__cast_u256_as_u8(i256 %0) {
entry:
  %local_0 = alloca i256, align 8
  %local_1 = alloca i256, align 8
  %local_2 = alloca i8, align 1
  store i256 %0, ptr %local_0, align 8
  %load_store_tmp = load i256, ptr %local_0, align 8
  store i256 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i256, ptr %local_1, align 8
  %castcond = icmp ugt i256 %cast_src, 255
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i256 %cast_src to i8
  store i8 %trunc_dst, ptr %local_2, align 1
  %retval = load i8, ptr %local_2, align 1
  ret i8 %retval
}

define private i128 @Test__cast_u32_as_u128(i32 %0) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i128, align 8
  store i32 %0, ptr %local_0, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_1, align 4
  %cast_src = load i32, ptr %local_1, align 4
  %zext_dst = zext i32 %cast_src to i128
  store i128 %zext_dst, ptr %local_2, align 8
  %retval = load i128, ptr %local_2, align 8
  ret i128 %retval
}

define private i16 @Test__cast_u32_as_u16(i32 %0) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i16, align 2
  store i32 %0, ptr %local_0, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_1, align 4
  %cast_src = load i32, ptr %local_1, align 4
  %castcond = icmp ugt i32 %cast_src, 65535
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i32 %cast_src to i16
  store i16 %trunc_dst, ptr %local_2, align 2
  %retval = load i16, ptr %local_2, align 2
  ret i16 %retval
}

define private i256 @Test__cast_u32_as_u256(i32 %0) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i256, align 8
  store i32 %0, ptr %local_0, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_1, align 4
  %cast_src = load i32, ptr %local_1, align 4
  %zext_dst = zext i32 %cast_src to i256
  store i256 %zext_dst, ptr %local_2, align 8
  %retval = load i256, ptr %local_2, align 8
  ret i256 %retval
}

define private i32 @Test__cast_u32_as_u32(i32 %0) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i32, align 4
  store i32 %0, ptr %local_0, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_1, align 4
  %cast_src = load i32, ptr %local_1, align 4
  store i32 %cast_src, ptr %local_2, align 4
  %retval = load i32, ptr %local_2, align 4
  ret i32 %retval
}

define private i64 @Test__cast_u32_as_u64(i32 %0) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i64, align 8
  store i32 %0, ptr %local_0, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_1, align 4
  %cast_src = load i32, ptr %local_1, align 4
  %zext_dst = zext i32 %cast_src to i64
  store i64 %zext_dst, ptr %local_2, align 8
  %retval = load i64, ptr %local_2, align 8
  ret i64 %retval
}

define private i8 @Test__cast_u32_as_u8(i32 %0) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i8, align 1
  store i32 %0, ptr %local_0, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_1, align 4
  %cast_src = load i32, ptr %local_1, align 4
  %castcond = icmp ugt i32 %cast_src, 255
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i32 %cast_src to i8
  store i8 %trunc_dst, ptr %local_2, align 1
  %retval = load i8, ptr %local_2, align 1
  ret i8 %retval
}

define private i128 @Test__cast_u64_as_u128(i64 %0) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i128, align 8
  store i64 %0, ptr %local_0, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i64, ptr %local_1, align 8
  %zext_dst = zext i64 %cast_src to i128
  store i128 %zext_dst, ptr %local_2, align 8
  %retval = load i128, ptr %local_2, align 8
  ret i128 %retval
}

define private i16 @Test__cast_u64_as_u16(i64 %0) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i16, align 2
  store i64 %0, ptr %local_0, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i64, ptr %local_1, align 8
  %castcond = icmp ugt i64 %cast_src, 65535
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i64 %cast_src to i16
  store i16 %trunc_dst, ptr %local_2, align 2
  %retval = load i16, ptr %local_2, align 2
  ret i16 %retval
}

define private i256 @Test__cast_u64_as_u256(i64 %0) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i256, align 8
  store i64 %0, ptr %local_0, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i64, ptr %local_1, align 8
  %zext_dst = zext i64 %cast_src to i256
  store i256 %zext_dst, ptr %local_2, align 8
  %retval = load i256, ptr %local_2, align 8
  ret i256 %retval
}

define private i32 @Test__cast_u64_as_u32(i64 %0) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i32, align 4
  store i64 %0, ptr %local_0, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i64, ptr %local_1, align 8
  %castcond = icmp ugt i64 %cast_src, 4294967295
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i64 %cast_src to i32
  store i32 %trunc_dst, ptr %local_2, align 4
  %retval = load i32, ptr %local_2, align 4
  ret i32 %retval
}

define private i64 @Test__cast_u64_as_u64(i64 %0) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i64, align 8
  store i64 %0, ptr %local_0, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i64, ptr %local_1, align 8
  store i64 %cast_src, ptr %local_2, align 8
  %retval = load i64, ptr %local_2, align 8
  ret i64 %retval
}

define private i8 @Test__cast_u64_as_u8(i64 %0) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i8, align 1
  store i64 %0, ptr %local_0, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_1, align 8
  %cast_src = load i64, ptr %local_1, align 8
  %castcond = icmp ugt i64 %cast_src, 255
  br i1 %castcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %trunc_dst = trunc i64 %cast_src to i8
  store i8 %trunc_dst, ptr %local_2, align 1
  %retval = load i8, ptr %local_2, align 1
  ret i8 %retval
}

define private i128 @Test__cast_u8_as_u128(i8 %0) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i128, align 8
  store i8 %0, ptr %local_0, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_1, align 1
  %cast_src = load i8, ptr %local_1, align 1
  %zext_dst = zext i8 %cast_src to i128
  store i128 %zext_dst, ptr %local_2, align 8
  %retval = load i128, ptr %local_2, align 8
  ret i128 %retval
}

define private i16 @Test__cast_u8_as_u16(i8 %0) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i16, align 2
  store i8 %0, ptr %local_0, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_1, align 1
  %cast_src = load i8, ptr %local_1, align 1
  %zext_dst = zext i8 %cast_src to i16
  store i16 %zext_dst, ptr %local_2, align 2
  %retval = load i16, ptr %local_2, align 2
  ret i16 %retval
}

define private i256 @Test__cast_u8_as_u256(i8 %0) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i256, align 8
  store i8 %0, ptr %local_0, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_1, align 1
  %cast_src = load i8, ptr %local_1, align 1
  %zext_dst = zext i8 %cast_src to i256
  store i256 %zext_dst, ptr %local_2, align 8
  %retval = load i256, ptr %local_2, align 8
  ret i256 %retval
}

define private i32 @Test__cast_u8_as_u32(i8 %0) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i32, align 4
  store i8 %0, ptr %local_0, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_1, align 1
  %cast_src = load i8, ptr %local_1, align 1
  %zext_dst = zext i8 %cast_src to i32
  store i32 %zext_dst, ptr %local_2, align 4
  %retval = load i32, ptr %local_2, align 4
  ret i32 %retval
}

define private i64 @Test__cast_u8_as_u64(i8 %0) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i64, align 8
  store i8 %0, ptr %local_0, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_1, align 1
  %cast_src = load i8, ptr %local_1, align 1
  %zext_dst = zext i8 %cast_src to i64
  store i64 %zext_dst, ptr %local_2, align 8
  %retval = load i64, ptr %local_2, align 8
  ret i64 %retval
}

define private i8 @Test__cast_u8_as_u8(i8 %0) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  store i8 %0, ptr %local_0, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_1, align 1
  %cast_src = load i8, ptr %local_1, align 1
  store i8 %cast_src, ptr %local_2, align 1
  %retval = load i8, ptr %local_2, align 1
  ret i8 %retval
}

; Function Attrs: noreturn
declare void @move_rt_abort(i64) #0

attributes #0 = { noreturn }
