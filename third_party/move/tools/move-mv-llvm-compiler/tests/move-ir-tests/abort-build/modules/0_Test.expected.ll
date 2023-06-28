; ModuleID = '0x100__Test'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define private void @Test__test() {
entry:
  %local_0 = alloca i64, align 8
  store i64 10, ptr %local_0, align 4
  %call_arg_0 = load i64, ptr %local_0, align 4
  call void @move_rt_abort(i64 %call_arg_0)
  unreachable
}

; Function Attrs: noreturn
declare void @move_rt_abort(i64) #0

attributes #0 = { noreturn }
