; ModuleID = '0x100__Test'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

declare i32 @memcmp(ptr, ptr, i64)

define private i8 @Test__get_sub(i8 %0, i8 %1) {
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

define private void @Test__test() {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i8, align 1
  %local_5 = alloca i8, align 1
  %local_6 = alloca i1, align 1
  %local_7 = alloca i64, align 8
  store i8 10, ptr %local_1, align 1
  store i8 3, ptr %local_2, align 1
  %call_arg_0 = load i8, ptr %local_1, align 1
  %call_arg_1 = load i8, ptr %local_2, align 1
  %retval = call i8 @Test__get_sub(i8 %call_arg_0, i8 %call_arg_1)
  store i8 %retval, ptr %local_3, align 1
  %load_store_tmp = load i8, ptr %local_3, align 1
  store i8 %load_store_tmp, ptr %local_0, align 1
  store i8 7, ptr %local_4, align 1
  %load_store_tmp1 = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp1, ptr %local_5, align 1
  %eq_src_0 = load i8, ptr %local_4, align 1
  %eq_src_1 = load i8, ptr %local_5, align 1
  %eq_dst = icmp eq i8 %eq_src_0, %eq_src_1
  store i1 %eq_dst, ptr %local_6, align 1
  %cnd = load i1, ptr %local_6, align 1
  br i1 %cnd, label %bb_1, label %bb_0

bb_1:                                             ; preds = %entry
  br label %bb_2

bb_0:                                             ; preds = %entry
  store i64 10, ptr %local_7, align 8
  %call_arg_02 = load i64, ptr %local_7, align 8
  call void @move_rt_abort(i64 %call_arg_02)
  unreachable

bb_2:                                             ; preds = %bb_1
  ret void
}

; Function Attrs: noreturn
declare void @move_rt_abort(i64) #0

attributes #0 = { noreturn }
