; ModuleID = '<SELF>'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

declare i32 @memcmp(ptr, ptr, i64)

define void @main() {
entry:
  %local_0 = alloca i8, align 1
  store i8 7, ptr %local_0, align 1
  ret void
}
