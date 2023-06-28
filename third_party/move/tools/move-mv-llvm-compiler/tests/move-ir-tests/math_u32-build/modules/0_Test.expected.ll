; ModuleID = '0x100__Test'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define private i32 @Test__test(i32 %0, i32 %1) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i32, align 4
  %local_3 = alloca i32, align 4
  %local_4 = alloca i32, align 4
  store i32 %0, ptr %local_0, align 4
  store i32 %1, ptr %local_1, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_2, align 4
  %load_store_tmp1 = load i32, ptr %local_1, align 4
  store i32 %load_store_tmp1, ptr %local_3, align 4
  %add_src_0 = load i32, ptr %local_2, align 4
  %add_src_1 = load i32, ptr %local_3, align 4
  %add_dst = add i32 %add_src_0, %add_src_1
  %ovfcond = icmp ult i32 %add_dst, %add_src_0
  br i1 %ovfcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i32 %add_dst, ptr %local_4, align 4
  %retval = load i32, ptr %local_4, align 4
  ret i32 %retval
}

define private i32 @Test__test_div(i32 %0, i32 %1) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i32, align 4
  %local_3 = alloca i32, align 4
  %local_4 = alloca i32, align 4
  store i32 %0, ptr %local_0, align 4
  store i32 %1, ptr %local_1, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_2, align 4
  %load_store_tmp1 = load i32, ptr %local_1, align 4
  store i32 %load_store_tmp1, ptr %local_3, align 4
  %div_src_0 = load i32, ptr %local_2, align 4
  %div_src_1 = load i32, ptr %local_3, align 4
  %zerocond = icmp eq i32 %div_src_1, 0
  br i1 %zerocond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %div_dst = udiv i32 %div_src_0, %div_src_1
  store i32 %div_dst, ptr %local_4, align 4
  %retval = load i32, ptr %local_4, align 4
  ret i32 %retval
}

define private i32 @Test__test_mul_trunc(i32 %0, i32 %1) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i32, align 4
  %local_3 = alloca i32, align 4
  %local_4 = alloca i32, align 4
  store i32 %0, ptr %local_0, align 4
  store i32 %1, ptr %local_1, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_2, align 4
  %load_store_tmp1 = load i32, ptr %local_1, align 4
  store i32 %load_store_tmp1, ptr %local_3, align 4
  %mul_src_0 = load i32, ptr %local_2, align 4
  %mul_src_1 = load i32, ptr %local_3, align 4
  %mul_val = call { i32, i1 } @llvm.umul.with.overflow.i32(i32 %mul_src_0, i32 %mul_src_1)
  %mul_dst = extractvalue { i32, i1 } %mul_val, 0
  %mul_ovf = extractvalue { i32, i1 } %mul_val, 1
  br i1 %mul_ovf, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i32 %mul_dst, ptr %local_4, align 4
  %retval = load i32, ptr %local_4, align 4
  ret i32 %retval
}

define private i32 @Test__test_sub(i32 %0, i32 %1) {
entry:
  %local_0 = alloca i32, align 4
  %local_1 = alloca i32, align 4
  %local_2 = alloca i32, align 4
  %local_3 = alloca i32, align 4
  %local_4 = alloca i32, align 4
  store i32 %0, ptr %local_0, align 4
  store i32 %1, ptr %local_1, align 4
  %load_store_tmp = load i32, ptr %local_0, align 4
  store i32 %load_store_tmp, ptr %local_2, align 4
  %load_store_tmp1 = load i32, ptr %local_1, align 4
  store i32 %load_store_tmp1, ptr %local_3, align 4
  %sub_src_0 = load i32, ptr %local_2, align 4
  %sub_src_1 = load i32, ptr %local_3, align 4
  %sub_dst = sub i32 %sub_src_0, %sub_src_1
  %ovfcond = icmp ugt i32 %sub_dst, %sub_src_0
  br i1 %ovfcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i32 %sub_dst, ptr %local_4, align 4
  %retval = load i32, ptr %local_4, align 4
  ret i32 %retval
}

; Function Attrs: noreturn
declare void @move_rt_abort(i64) #0

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i32, i1 } @llvm.umul.with.overflow.i32(i32, i32) #1

attributes #0 = { noreturn }
attributes #1 = { nocallback nofree nosync nounwind readnone speculatable willreturn }
