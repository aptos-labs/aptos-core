; ModuleID = '<SELF>'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define void @main() {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  store i8 1, ptr %local_0, align 1
  store i8 2, ptr %local_1, align 1
  %call_arg_0 = load i8, ptr %local_0, align 1
  %call_arg_1 = load i8, ptr %local_1, align 1
  %retval = call i8 @Test1__test1(i8 %call_arg_0, i8 %call_arg_1)
  store i8 %retval, ptr %local_2, align 1
  ret void
}

declare i8 @Test1__test1(i8, i8)
