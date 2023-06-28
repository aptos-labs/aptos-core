; ModuleID = '0x200__M11'
source_filename = "<unknown>"
target datalayout = "e-m:e-p:64:64-i64:64-n32:64-S128"
target triple = "sbf-solana-solana"

%struct.Coins__Coin_M11__USDC_ = type { i64, i8 }
%struct.Coins__Coin_M11__Eth_ = type { i64, i8 }

declare i32 @memcmp(ptr, ptr, i64)

define private i64 @M11__get_value_usdc(%struct.Coins__Coin_M11__USDC_ %0) {
entry:
  %local_0 = alloca %struct.Coins__Coin_M11__USDC_, align 8
  %local_1 = alloca %struct.Coins__Coin_M11__USDC_, align 8
  %local_2 = alloca i64, align 8
  store %struct.Coins__Coin_M11__USDC_ %0, ptr %local_0, align 8
  %call_arg_0 = load %struct.Coins__Coin_M11__USDC_, ptr %local_0, align 8
  %retval = call i64 @Coins__get_value_generic_M11__USDC(%struct.Coins__Coin_M11__USDC_ %call_arg_0)
  store i64 %retval, ptr %local_2, align 8
  %retval1 = load i64, ptr %local_2, align 8
  ret i64 %retval1
}

define private i64 @Coins__get_value_generic_M11__USDC(%struct.Coins__Coin_M11__USDC_ %0) {
entry:
  %local_0 = alloca %struct.Coins__Coin_M11__USDC_, align 8
  %local_1 = alloca %struct.Coins__Coin_M11__USDC_, align 8
  %local_2__value = alloca i64, align 8
  store %struct.Coins__Coin_M11__USDC_ %0, ptr %local_0, align 8
  %srcval = load %struct.Coins__Coin_M11__USDC_, ptr %local_0, align 8
  %ext_0 = extractvalue %struct.Coins__Coin_M11__USDC_ %srcval, 0
  store i64 %ext_0, ptr %local_2__value, align 8
  %retval = load i64, ptr %local_2__value, align 8
  ret i64 %retval
}

define private { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } @M11__mint_2coins_usdc_and_eth(i64 %0, i64 %1) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca i64, align 8
  %local_3 = alloca i64, align 8
  %local_4 = alloca %struct.Coins__Coin_M11__USDC_, align 8
  %local_5 = alloca %struct.Coins__Coin_M11__Eth_, align 8
  store i64 %0, ptr %local_0, align 8
  store i64 %1, ptr %local_1, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_2, align 8
  %load_store_tmp1 = load i64, ptr %local_1, align 8
  store i64 %load_store_tmp1, ptr %local_3, align 8
  %call_arg_0 = load i64, ptr %local_2, align 8
  %call_arg_1 = load i64, ptr %local_3, align 8
  %retval = call { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } @Coins__mint_2coins_generic_M11__USDC_M11__Eth(i64 %call_arg_0, i64 %call_arg_1)
  %extract_0 = extractvalue { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } %retval, 0
  %extract_1 = extractvalue { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } %retval, 1
  store %struct.Coins__Coin_M11__USDC_ %extract_0, ptr %local_4, align 8
  store %struct.Coins__Coin_M11__Eth_ %extract_1, ptr %local_5, align 8
  %rv.0 = load %struct.Coins__Coin_M11__USDC_, ptr %local_4, align 8
  %rv.1 = load %struct.Coins__Coin_M11__Eth_, ptr %local_5, align 8
  %insert_0 = insertvalue { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } undef, %struct.Coins__Coin_M11__USDC_ %rv.0, 0
  %insert_1 = insertvalue { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } %insert_0, %struct.Coins__Coin_M11__Eth_ %rv.1, 1
  ret { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } %insert_1
}

define private { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } @Coins__mint_2coins_generic_M11__USDC_M11__Eth(i64 %0, i64 %1) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2__value = alloca i64, align 8
  %local_3 = alloca %struct.Coins__Coin_M11__USDC_, align 8
  %local_4__value = alloca i64, align 8
  %local_5 = alloca %struct.Coins__Coin_M11__Eth_, align 8
  store i64 %0, ptr %local_0, align 8
  store i64 %1, ptr %local_1, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_2__value, align 8
  %fv.0 = load i64, ptr %local_2__value, align 8
  %insert_0 = insertvalue %struct.Coins__Coin_M11__USDC_ undef, i64 %fv.0, 0
  store %struct.Coins__Coin_M11__USDC_ %insert_0, ptr %local_3, align 8
  %load_store_tmp1 = load i64, ptr %local_1, align 8
  store i64 %load_store_tmp1, ptr %local_4__value, align 8
  %fv.02 = load i64, ptr %local_4__value, align 8
  %insert_03 = insertvalue %struct.Coins__Coin_M11__Eth_ undef, i64 %fv.02, 0
  store %struct.Coins__Coin_M11__Eth_ %insert_03, ptr %local_5, align 8
  %rv.0 = load %struct.Coins__Coin_M11__USDC_, ptr %local_3, align 8
  %rv.1 = load %struct.Coins__Coin_M11__Eth_, ptr %local_5, align 8
  %insert_04 = insertvalue { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } undef, %struct.Coins__Coin_M11__USDC_ %rv.0, 0
  %insert_1 = insertvalue { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } %insert_04, %struct.Coins__Coin_M11__Eth_ %rv.1, 1
  ret { %struct.Coins__Coin_M11__USDC_, %struct.Coins__Coin_M11__Eth_ } %insert_1
}

define private %struct.Coins__Coin_M11__USDC_ @M11__mint_usdc(i64 %0) {
entry:
  %local_0 = alloca i64, align 8
  %local_1 = alloca i64, align 8
  %local_2 = alloca %struct.Coins__Coin_M11__USDC_, align 8
  store i64 %0, ptr %local_0, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_1, align 8
  %call_arg_0 = load i64, ptr %local_1, align 8
  %retval = call %struct.Coins__Coin_M11__USDC_ @Coins__mint_generic_M11__USDC(i64 %call_arg_0)
  store %struct.Coins__Coin_M11__USDC_ %retval, ptr %local_2, align 8
  %retval1 = load %struct.Coins__Coin_M11__USDC_, ptr %local_2, align 8
  ret %struct.Coins__Coin_M11__USDC_ %retval1
}

define private %struct.Coins__Coin_M11__USDC_ @Coins__mint_generic_M11__USDC(i64 %0) {
entry:
  %local_0 = alloca i64, align 8
  %local_1__value = alloca i64, align 8
  %local_2 = alloca %struct.Coins__Coin_M11__USDC_, align 8
  store i64 %0, ptr %local_0, align 8
  %load_store_tmp = load i64, ptr %local_0, align 8
  store i64 %load_store_tmp, ptr %local_1__value, align 8
  %fv.0 = load i64, ptr %local_1__value, align 8
  %insert_0 = insertvalue %struct.Coins__Coin_M11__USDC_ undef, i64 %fv.0, 0
  store %struct.Coins__Coin_M11__USDC_ %insert_0, ptr %local_2, align 8
  %retval = load %struct.Coins__Coin_M11__USDC_, ptr %local_2, align 8
  ret %struct.Coins__Coin_M11__USDC_ %retval
}
