; ModuleID = '0x100__Test'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define private i8 @Test__test_add(i8 %0, i8 %1) {
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
  %add_src_0 = load i8, ptr %local_2, align 1
  %add_src_1 = load i8, ptr %local_3, align 1
  %add_dst = add i8 %add_src_0, %add_src_1
  %ovfcond = icmp ult i8 %add_dst, %add_src_0
  br i1 %ovfcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i8 %add_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

define private i8 @Test__test_div(i8 %0, i8 %1) {
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
  %div_src_0 = load i8, ptr %local_2, align 1
  %div_src_1 = load i8, ptr %local_3, align 1
  %zerocond = icmp eq i8 %div_src_1, 0
  br i1 %zerocond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %div_dst = udiv i8 %div_src_0, %div_src_1
  store i8 %div_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

define private i8 @Test__test_mod(i8 %0, i8 %1) {
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
  %mod_src_0 = load i8, ptr %local_2, align 1
  %mod_src_1 = load i8, ptr %local_3, align 1
  %zerocond = icmp eq i8 %mod_src_1, 0
  br i1 %zerocond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  %mod_dst = urem i8 %mod_src_0, %mod_src_1
  store i8 %mod_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

define private i8 @Test__test_mul(i8 %0, i8 %1) {
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
  %mul_src_0 = load i8, ptr %local_2, align 1
  %mul_src_1 = load i8, ptr %local_3, align 1
  %mul_val = call { i8, i1 } @llvm.umul.with.overflow.i8(i8 %mul_src_0, i8 %mul_src_1)
  %mul_dst = extractvalue { i8, i1 } %mul_val, 0
  %mul_ovf = extractvalue { i8, i1 } %mul_val, 1
  br i1 %mul_ovf, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i8 %mul_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

define private i8 @Test__test_sub(i8 %0, i8 %1) {
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
  %sub_src_0 = load i8, ptr %local_2, align 1
  %sub_src_1 = load i8, ptr %local_3, align 1
  %sub_dst = sub i8 %sub_src_0, %sub_src_1
  %ovfcond = icmp ugt i8 %sub_dst, %sub_src_0
  br i1 %ovfcond, label %then_bb, label %join_bb

then_bb:                                          ; preds = %entry
  call void @move_rt_abort(i64 4017)
  unreachable

join_bb:                                          ; preds = %entry
  store i8 %sub_dst, ptr %local_4, align 1
  %retval = load i8, ptr %local_4, align 1
  ret i8 %retval
}

; Function Attrs: noreturn
declare void @move_rt_abort(i64) #0

; Function Attrs: nocallback nofree nosync nounwind readnone speculatable willreturn
declare { i8, i1 } @llvm.umul.with.overflow.i8(i8, i8) #1

attributes #0 = { noreturn }
attributes #1 = { nocallback nofree nosync nounwind readnone speculatable willreturn }
