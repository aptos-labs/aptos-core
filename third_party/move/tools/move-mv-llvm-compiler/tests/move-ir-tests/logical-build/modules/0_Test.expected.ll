; ModuleID = '0x100__Test'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define private i1 @Test__test_eq(i8 %0, i8 %1) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i1, align 1
  store i8 %0, ptr %local_0, align 1
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %eq_src_0 = load i8, ptr %local_2, align 1
  %eq_src_1 = load i8, ptr %local_3, align 1
  %eq_dst = icmp eq i8 %eq_src_0, %eq_src_1
  store i1 %eq_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define private i1 @Test__test_ge(i8 %0, i8 %1) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i1, align 1
  store i8 %0, ptr %local_0, align 1
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %ge_src_0 = load i8, ptr %local_2, align 1
  %ge_src_1 = load i8, ptr %local_3, align 1
  %ge_dst = icmp uge i8 %ge_src_0, %ge_src_1
  store i1 %ge_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define private i1 @Test__test_gt(i8 %0, i8 %1) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i1, align 1
  store i8 %0, ptr %local_0, align 1
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %gt_src_0 = load i8, ptr %local_2, align 1
  %gt_src_1 = load i8, ptr %local_3, align 1
  %gt_dst = icmp ugt i8 %gt_src_0, %gt_src_1
  store i1 %gt_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define private i1 @Test__test_le(i8 %0, i8 %1) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i1, align 1
  store i8 %0, ptr %local_0, align 1
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %le_src_0 = load i8, ptr %local_2, align 1
  %le_src_1 = load i8, ptr %local_3, align 1
  %le_dst = icmp ule i8 %le_src_0, %le_src_1
  store i1 %le_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define private i1 @Test__test_logical_and(i1 %0, i1 %1) {
entry:
  %local_0 = alloca i1, align 1
  %local_1 = alloca i1, align 1
  %local_2 = alloca i1, align 1
  %local_3 = alloca i1, align 1
  %local_4 = alloca i1, align 1
  store i1 %0, ptr %local_0, align 1
  store i1 %1, ptr %local_1, align 1
  %load_store_tmp = load i1, ptr %local_0, align 1
  store i1 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i1, ptr %local_1, align 1
  store i1 %load_store_tmp1, ptr %local_3, align 1
  %and_src_0 = load i1, ptr %local_2, align 1
  %and_src_1 = load i1, ptr %local_3, align 1
  %and_dst = and i1 %and_src_0, %and_src_1
  store i1 %and_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define private i1 @Test__test_logical_or(i1 %0, i1 %1) {
entry:
  %local_0 = alloca i1, align 1
  %local_1 = alloca i1, align 1
  %local_2 = alloca i1, align 1
  %local_3 = alloca i1, align 1
  %local_4 = alloca i1, align 1
  store i1 %0, ptr %local_0, align 1
  store i1 %1, ptr %local_1, align 1
  %load_store_tmp = load i1, ptr %local_0, align 1
  store i1 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i1, ptr %local_1, align 1
  store i1 %load_store_tmp1, ptr %local_3, align 1
  %or_src_0 = load i1, ptr %local_2, align 1
  %or_src_1 = load i1, ptr %local_3, align 1
  %or_dst = or i1 %or_src_0, %or_src_1
  store i1 %or_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define private i1 @Test__test_lt(i8 %0, i8 %1) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i1, align 1
  store i8 %0, ptr %local_0, align 1
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %lt_src_0 = load i8, ptr %local_2, align 1
  %lt_src_1 = load i8, ptr %local_3, align 1
  %lt_dst = icmp ult i8 %lt_src_0, %lt_src_1
  store i1 %lt_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define private i1 @Test__test_ne(i8 %0, i8 %1) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca i8, align 1
  %local_4 = alloca i1, align 1
  store i8 %0, ptr %local_0, align 1
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp1, ptr %local_3, align 1
  %ne_src_0 = load i8, ptr %local_2, align 1
  %ne_src_1 = load i8, ptr %local_3, align 1
  %ne_dst = icmp ne i8 %ne_src_0, %ne_src_1
  store i1 %ne_dst, ptr %local_4, align 1
  %retval = load i1, ptr %local_4, align 1
  ret i1 %retval
}

define private i1 @Test__test_not(i1 %0) {
entry:
  %local_0 = alloca i1, align 1
  %local_1 = alloca i1, align 1
  %local_2 = alloca i1, align 1
  store i1 %0, ptr %local_0, align 1
  %load_store_tmp = load i1, ptr %local_0, align 1
  store i1 %load_store_tmp, ptr %local_1, align 1
  %not_src = load i1, ptr %local_1, align 1
  %not_dst = xor i1 %not_src, true
  store i1 %not_dst, ptr %local_2, align 1
  %retval = load i1, ptr %local_2, align 1
  ret i1 %retval
}
