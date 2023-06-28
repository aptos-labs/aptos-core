; ModuleID = '0x100__Test'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define private i8 @Test__test() {
entry:
  %local_0 = alloca i8, align 1
  store i8 100, ptr %local_0, align 1
  %retval = load i8, ptr %local_0, align 1
  ret i8 %retval
}
