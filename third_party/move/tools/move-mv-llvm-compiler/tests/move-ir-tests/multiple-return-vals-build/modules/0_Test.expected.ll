; ModuleID = '0x100__Test'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define private { i1, i1 } @Test__ret_2vals() {
entry:
  %local_0 = alloca i1, align 1
  %local_1 = alloca i1, align 1
  store i1 true, ptr %local_0, align 1
  store i1 false, ptr %local_1, align 1
  %rv.0 = load i1, ptr %local_0, align 1
  %rv.1 = load i1, ptr %local_1, align 1
  %insert_0 = insertvalue { i1, i1 } undef, i1 %rv.0, 0
  %insert_1 = insertvalue { i1, i1 } %insert_0, i1 %rv.1, 1
  ret { i1, i1 } %insert_1
}

define private { ptr, i8, i128, i32 } @Test__ret_4vals(ptr %0) {
entry:
  %local_0 = alloca ptr, align 8
  %local_1 = alloca ptr, align 8
  %local_2 = alloca i8, align 1
  %local_3 = alloca i128, align 8
  %local_4 = alloca i32, align 4
  store ptr %0, ptr %local_0, align 8
  %load_store_tmp = load ptr, ptr %local_0, align 8
  store ptr %load_store_tmp, ptr %local_1, align 8
  store i8 8, ptr %local_2, align 1
  store i128 128, ptr %local_3, align 4
  store i32 32, ptr %local_4, align 4
  %rv.0 = load ptr, ptr %local_1, align 8
  %rv.1 = load i8, ptr %local_2, align 1
  %rv.2 = load i128, ptr %local_3, align 4
  %rv.3 = load i32, ptr %local_4, align 4
  %insert_0 = insertvalue { ptr, i8, i128, i32 } undef, ptr %rv.0, 0
  %insert_1 = insertvalue { ptr, i8, i128, i32 } %insert_0, i8 %rv.1, 1
  %insert_2 = insertvalue { ptr, i8, i128, i32 } %insert_1, i128 %rv.2, 2
  %insert_3 = insertvalue { ptr, i8, i128, i32 } %insert_2, i32 %rv.3, 3
  ret { ptr, i8, i128, i32 } %insert_3
}

define private void @Test__use_2val_call_result() {
entry:
  %local_0 = alloca i1, align 1
  %local_1 = alloca i1, align 1
  %local_2 = alloca i1, align 1
  %retval = call { i1, i1 } @Test__ret_2vals()
  %extract_0 = extractvalue { i1, i1 } %retval, 0
  %extract_1 = extractvalue { i1, i1 } %retval, 1
  store i1 %extract_0, ptr %local_0, align 1
  store i1 %extract_1, ptr %local_1, align 1
  %or_src_0 = load i1, ptr %local_0, align 1
  %or_src_1 = load i1, ptr %local_1, align 1
  %or_dst = or i1 %or_src_0, %or_src_1
  store i1 %or_dst, ptr %local_2, align 1
  ret void
}

define private void @Test__use_4val_call_result() {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i8, align 1
  %local_2 = alloca i128, align 8
  %local_3 = alloca i32, align 4
  %local_4 = alloca i64, align 8
  %local_5 = alloca ptr, align 8
  %local_6 = alloca ptr, align 8
  %local_7 = alloca i8, align 1
  %local_8 = alloca i128, align 8
  %local_9 = alloca i32, align 4
  %local_10 = alloca i64, align 8
  %local_11 = alloca i8, align 1
  %local_12 = alloca i128, align 8
  %local_13 = alloca i32, align 4
  store i64 0, ptr %local_4, align 4
  %load_store_tmp = load i64, ptr %local_4, align 4
  store i64 %load_store_tmp, ptr %local_0, align 4
  store ptr %local_0, ptr %local_5, align 8
  %call_arg_0 = load ptr, ptr %local_5, align 8
  %retval = call { ptr, i8, i128, i32 } @Test__ret_4vals(ptr %call_arg_0)
  %extract_0 = extractvalue { ptr, i8, i128, i32 } %retval, 0
  %extract_1 = extractvalue { ptr, i8, i128, i32 } %retval, 1
  %extract_2 = extractvalue { ptr, i8, i128, i32 } %retval, 2
  %extract_3 = extractvalue { ptr, i8, i128, i32 } %retval, 3
  store ptr %extract_0, ptr %local_6, align 8
  store i8 %extract_1, ptr %local_7, align 1
  store i128 %extract_2, ptr %local_8, align 4
  store i32 %extract_3, ptr %local_9, align 4
  %load_store_tmp1 = load i32, ptr %local_9, align 4
  store i32 %load_store_tmp1, ptr %local_3, align 4
  %load_store_tmp2 = load i128, ptr %local_8, align 4
  store i128 %load_store_tmp2, ptr %local_2, align 4
  %load_store_tmp3 = load i8, ptr %local_7, align 1
  store i8 %load_store_tmp3, ptr %local_1, align 1
  %load_deref_store_tmp1 = load ptr, ptr %local_6, align 8
  %load_deref_store_tmp2 = load i64, ptr %load_deref_store_tmp1, align 4
  store i64 %load_deref_store_tmp2, ptr %local_10, align 4
  %load_store_tmp4 = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp4, ptr %local_11, align 1
  %load_store_tmp5 = load i128, ptr %local_2, align 4
  store i128 %load_store_tmp5, ptr %local_12, align 4
  %load_store_tmp6 = load i32, ptr %local_3, align 4
  store i32 %load_store_tmp6, ptr %local_13, align 4
  ret void
}
