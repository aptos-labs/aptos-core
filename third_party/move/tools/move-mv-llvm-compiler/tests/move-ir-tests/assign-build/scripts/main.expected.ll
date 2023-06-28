; ModuleID = '<SELF>'
source_filename = "<unknown>"

declare i32 @memcmp(ptr, ptr, i64)

define void @main() {
entry:
  %local_0 = alloca i8, align 1
  store i8 7, ptr %local_0, align 1
  ret void
}
