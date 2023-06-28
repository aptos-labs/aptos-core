; ModuleID = '0x100__Test'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define private i8 @Test__test(i1 %0) {
entry:
  %local_0 = alloca i1, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i1, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i8, align 1
  %local_5 = alloca i8, align 1
  store i1 %0, ptr %local_0, align 1
  %load_store_tmp = load i1, ptr %local_0, align 1
  store i1 %load_store_tmp, ptr %local_2, align 1
  %cnd = load i1, ptr %local_2, align 1
  br i1 %cnd, label %bb_1, label %bb_0

bb_1:                                             ; preds = %entry
  store i8 2, ptr %local_3, align 1
  %load_store_tmp1 = load i8, ptr %local_3, align 1
  store i8 %load_store_tmp1, ptr %local_1, align 1
  br label %bb_2

bb_0:                                             ; preds = %entry
  store i8 3, ptr %local_4, align 1
  %load_store_tmp2 = load i8, ptr %local_4, align 1
  store i8 %load_store_tmp2, ptr %local_1, align 1
  br label %bb_2

bb_2:                                             ; preds = %bb_0, %bb_1
  %load_store_tmp3 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp3, ptr %local_5, align 1
  %retval = load i8, ptr %local_5, align 1
  ret i8 %retval
}
