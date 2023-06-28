; ModuleID = '0x100__M'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

%struct.M__MyStruct = type { i32, i1, %struct.M__EmptyStruct, i8 }
%struct.M__EmptyStruct = type { i1, i8 }

declare i32 @memcmp(ptr, ptr, i64)

define %struct.M__MyStruct @M__boofun() {
entry:
  %local_0__field1 = alloca i32, align 4
  %local_1__field2 = alloca i1, align 1
  %local_2__dummy_field = alloca i1, align 1
  %local_3__field3 = alloca %struct.M__EmptyStruct, align 8
  %local_4 = alloca %struct.M__MyStruct, align 8
  store i32 32, ptr %local_0__field1, align 4
  store i1 true, ptr %local_1__field2, align 1
  store i1 false, ptr %local_2__dummy_field, align 1
  %fv.0 = load i1, ptr %local_2__dummy_field, align 1
  %insert_0 = insertvalue %struct.M__EmptyStruct undef, i1 %fv.0, 0
  store %struct.M__EmptyStruct %insert_0, ptr %local_3__field3, align 1
  %fv.01 = load i32, ptr %local_0__field1, align 4
  %fv.1 = load i1, ptr %local_1__field2, align 1
  %fv.2 = load %struct.M__EmptyStruct, ptr %local_3__field3, align 1
  %insert_02 = insertvalue %struct.M__MyStruct undef, i32 %fv.01, 0
  %insert_1 = insertvalue %struct.M__MyStruct %insert_02, i1 %fv.1, 1
  %insert_2 = insertvalue %struct.M__MyStruct %insert_1, %struct.M__EmptyStruct %fv.2, 2
  store %struct.M__MyStruct %insert_2, ptr %local_4, align 4
  %retval = load %struct.M__MyStruct, ptr %local_4, align 4
  ret %struct.M__MyStruct %retval
}
