; ModuleID = '0x100__Test'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

declare i32 @memcmp(ptr, ptr, i64)

define private i64 @Test__test(i64 %0, i64 %1) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i64, align 8
  %local_3 = alloca i64, align 8
  %local_4 = alloca i64, align 8
  store i64 %0, ptr %local_0, align 8
  store i64 %1, ptr %local_1, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i64, ptr %local_1, align 8
  store i64 %load_store_tmp1, ptr %local_3, align 8
  %add_src_0 = load i64, ptr %local_2, align 8
  %add_src_1 = load i64, ptr %local_3, align 8
  %add_dst = add i64 %add_src_0, %add_src_1
  %ovfcond = icmp ult i64 %add_dst, %add_src_0
  br i1 %ovfcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i64 %add_dst, ptr %local_4, align 8
  %retval = load i64, ptr %local_4, align 8
  ret i64 %retval
}

define private i64 @Test__test_div(i64 %0, i64 %1) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i64, align 8
  %local_3 = alloca i64, align 8
  %local_4 = alloca i64, align 8
  store i64 %0, ptr %local_0, align 8
  store i64 %1, ptr %local_1, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i64, ptr %local_1, align 8
  store i64 %load_store_tmp1, ptr %local_3, align 8
  %div_src_0 = load i64, ptr %local_2, align 8
  %div_src_1 = load i64, ptr %local_3, align 8
  %zerocond = icmp eq i64 %div_src_1, 0
  br i1 %zerocond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %div_dst = udiv i64 %div_src_0, %div_src_1
  store i64 %div_dst, ptr %local_4, align 8
  %retval = load i64, ptr %local_4, align 8
  ret i64 %retval
}

define private i64 @Test__test_mul(i64 %0, i64 %1) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i64, align 8
  %local_3 = alloca i64, align 8
  %local_4 = alloca i64, align 8
  store i64 %0, ptr %local_0, align 8
  store i64 %1, ptr %local_1, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i64, ptr %local_1, align 8
  store i64 %load_store_tmp1, ptr %local_3, align 8
  %mul_src_0 = load i64, ptr %local_2, align 8
  %mul_src_1 = load i64, ptr %local_3, align 8
  %mul_val = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %mul_src_0, i64 %mul_src_1)
  %mul_dst = extractvalue { i64, i1 } %mul_val, 0
  %mul_ovf = extractvalue { i64, i1 } %mul_val, 1
  br i1 %mul_ovf, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i64 %mul_dst, ptr %local_4, align 8
  %retval = load i64, ptr %local_4, align 8
  ret i64 %retval
}

define private i64 @Test__test_sub(i64 %0, i64 %1) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i64, align 8
  %local_3 = alloca i64, align 8
  %local_4 = alloca i64, align 8
  store i64 %0, ptr %local_0, align 8
  store i64 %1, ptr %local_1, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i64, ptr %local_1, align 8
  store i64 %load_store_tmp1, ptr %local_3, align 8
  %sub_src_0 = load i64, ptr %local_2, align 8
  %sub_src_1 = load i64, ptr %local_3, align 8
  %sub_dst = sub i64 %sub_src_0, %sub_src_1
  %ovfcond = icmp ugt i64 %sub_dst, %sub_src_0
  br i1 %ovfcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i64 %sub_dst, ptr %local_4, align 8
  %retval = load i64, ptr %local_4, align 8
  ret i64 %retval
}

; Function Attrs: noreturn
declare void @move_rt_abort(i64) #0

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i64, i1 } @llvm.umul.with.overflow.i64(i64, i64) #1

attributes #0 = { noreturn }
attributes #1 = { nocallback nofree nosync nounwind readnone speculatable willreturn }
