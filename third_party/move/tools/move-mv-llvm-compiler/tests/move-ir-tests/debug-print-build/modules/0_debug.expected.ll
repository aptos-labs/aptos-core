; ModuleID = '0x10__debug'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

declare i32 @memcmp(ptr, ptr, i64)

declare void @move_native_debug_print(ptr, ptr)
