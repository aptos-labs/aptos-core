; ModuleID = '0x200__UseIt'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

%struct.Country__Country = type { i8, i64, %struct.Country__Dunno, i8 }
%struct.Country__Dunno = type { i64, i8 }

declare i32 @memcmp(ptr, ptr, i64)

define void @UseIt__getit() {
entry:
  %local_0 = alloca %struct.Country__Country, align 8
  %local_1 = alloca i8, align 1
  %local_2 = alloca i64, align 8
  %local_3 = alloca %struct.Country__Country, align 8
  %local_4 = alloca %struct.Country__Country, align 8
  %local_5 = alloca i64, align 8
  %local_6 = alloca ptr, align 8
  %local_7 = alloca i8, align 1
  %local_8 = alloca ptr, align 8
  %local_9 = alloca i8, align 1
  %local_10 = alloca %struct.Country__Country, align 8
  %local_11 = alloca i8, align 1
  store i8 1, ptr %local_1, align 1
  store i64 1000000, ptr %local_2, align 8
  %call_arg_0 = load i8, ptr %local_1, align 1
  %call_arg_1 = load i64, ptr %local_2, align 8
  %retval = call %struct.Country__Country @Country__new_country(i8 %call_arg_0, i64 %call_arg_1)
  store %struct.Country__Country %retval, ptr %local_3, align 8
  %load_store_tmp = load %struct.Country__Country, ptr %local_3, align 8
  store %struct.Country__Country %load_store_tmp, ptr %local_0, align 8
  %load_store_tmp1 = load %struct.Country__Country, ptr %local_0, align 8
  store %struct.Country__Country %load_store_tmp1, ptr %local_4, align 8
  %call_arg_02 = load %struct.Country__Country, ptr %local_4, align 8
  %retval3 = call i64 @Country__get_pop(%struct.Country__Country %call_arg_02)
  store i64 %retval3, ptr %local_5, align 8
  store ptr %local_0, ptr %local_6, align 8
  %call_arg_04 = load ptr, ptr %local_6, align 8
  %retval5 = call i8 @Country__get_id(ptr %call_arg_04)
  store i8 %retval5, ptr %local_7, align 1
  store ptr %local_0, ptr %local_8, align 8
  store i8 123, ptr %local_9, align 1
  %call_arg_06 = load ptr, ptr %local_8, align 8
  %call_arg_17 = load i8, ptr %local_9, align 1
  call void @Country__set_id(ptr %call_arg_06, i8 %call_arg_17)
  %call_arg_08 = load %struct.Country__Country, ptr %local_0, align 8
  %retval9 = call i8 @Country__dropit(%struct.Country__Country %call_arg_08)
  store i8 %retval9, ptr %local_11, align 1
  ret void
}

declare %struct.Country__Country @Country__new_country(i8, i64)

declare i64 @Country__get_pop(%struct.Country__Country)

declare i8 @Country__get_id(ptr)

declare void @Country__set_id(ptr, i8)

declare i8 @Country__dropit(%struct.Country__Country)
