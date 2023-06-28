; ModuleID = '0x100__M6'
source_filename = "<unknown>"

%struct.M6__Foo_bool_ = type { i1, i8 }
%struct.M6__Bar_u8.u64_ = type { i8, i64, i8 }
%struct.M6__Baz_address.u32_ = type { [32 x i8], %struct.M6__Foo_u32_, i8 }
%struct.M6__Foo_u32_ = type { i32, i8 }
%struct.M6__Foo_u16_ = type { i16, i8 }
%struct.M6__Foo_u64_ = type { i64, i8 }
%struct.M6__Baz_u8.u64_ = type { i8, %struct.M6__Foo_u64_, i8 }

declare i32 @memcmp(ptr, ptr, i64)

define private i1 @M6__boo() {
entry:
  %local_0__x = alloca i1, align 1
  %local_1 = alloca %struct.M6__Foo_bool_, align 8
  %local_2__x = alloca i1, align 1
  store i1 true, ptr %local_0__x, align 1
  %fv.0 = load i1, ptr %local_0__x, align 1
  %insert_0 = insertvalue %struct.M6__Foo_bool_ undef, i1 %fv.0, 0
  store %struct.M6__Foo_bool_ %insert_0, ptr %local_1, align 1
  %srcval = load %struct.M6__Foo_bool_, ptr %local_1, align 1
  %ext_0 = extractvalue %struct.M6__Foo_bool_ %srcval, 0
  store i1 %ext_0, ptr %local_2__x, align 1
  %retval = load i1, ptr %local_2__x, align 1
  ret i1 %retval
}

define private { i8, i64 } @M6__goo() {
entry:
  %local_0__x = alloca i8, align 1
  %local_1__y = alloca i64, align 8
  %local_2 = alloca %struct.M6__Bar_u8.u64_, align 8
  %local_3__x = alloca i8, align 1
  %local_4__y = alloca i64, align 8
  store i8 123, ptr %local_0__x, align 1
  store i64 456, ptr %local_1__y, align 4
  %fv.0 = load i8, ptr %local_0__x, align 1
  %fv.1 = load i64, ptr %local_1__y, align 4
  %insert_0 = insertvalue %struct.M6__Bar_u8.u64_ undef, i8 %fv.0, 0
  %insert_1 = insertvalue %struct.M6__Bar_u8.u64_ %insert_0, i64 %fv.1, 1
  store %struct.M6__Bar_u8.u64_ %insert_1, ptr %local_2, align 4
  %srcval = load %struct.M6__Bar_u8.u64_, ptr %local_2, align 4
  %ext_0 = extractvalue %struct.M6__Bar_u8.u64_ %srcval, 0
  %ext_1 = extractvalue %struct.M6__Bar_u8.u64_ %srcval, 1
  store i8 %ext_0, ptr %local_3__x, align 1
  store i64 %ext_1, ptr %local_4__y, align 4
  %rv.0 = load i8, ptr %local_3__x, align 1
  %rv.1 = load i64, ptr %local_4__y, align 4
  %insert_01 = insertvalue { i8, i64 } undef, i8 %rv.0, 0
  %insert_12 = insertvalue { i8, i64 } %insert_01, i64 %rv.1, 1
  ret { i8, i64 } %insert_12
}

define private i32 @M6__rcv_and_idx(%struct.M6__Baz_address.u32_ %0) {
entry:
  %local_0 = alloca %struct.M6__Baz_address.u32_, align 8
  %local_1 = alloca ptr, align 8
  %local_2__y = alloca ptr, align 8
  %local_3__x = alloca ptr, align 8
  %local_4 = alloca i32, align 4
  store %struct.M6__Baz_address.u32_ %0, ptr %local_0, align 4
  store ptr %local_0, ptr %local_1, align 8
  %tmp = load ptr, ptr %local_1, align 8
  %fld_ref = getelementptr inbounds %struct.M6__Baz_address.u32_, ptr %tmp, i32 0, i32 1
  store ptr %fld_ref, ptr %local_2__y, align 8
  %tmp1 = load ptr, ptr %local_2__y, align 8
  %fld_ref2 = getelementptr inbounds %struct.M6__Foo_u32_, ptr %tmp1, i32 0, i32 0
  store ptr %fld_ref2, ptr %local_3__x, align 8
  %load_deref_store_tmp1 = load ptr, ptr %local_3__x, align 8
  %load_deref_store_tmp2 = load i32, ptr %load_deref_store_tmp1, align 4
  store i32 %load_deref_store_tmp2, ptr %local_4, align 4
  %retval = load i32, ptr %local_4, align 4
  ret i32 %retval
}

define private %struct.M6__Foo_u16_ @M6__snd_rcv(%struct.M6__Foo_u16_ %0) {
entry:
  %local_0 = alloca %struct.M6__Foo_u16_, align 8
  %local_1 = alloca %struct.M6__Foo_u16_, align 8
  store %struct.M6__Foo_u16_ %0, ptr %local_0, align 2
  %retval = load %struct.M6__Foo_u16_, ptr %local_0, align 2
  ret %struct.M6__Foo_u16_ %retval
}

define private { i8, i64 } @M6__zoo() {
entry:
  %local_0 = alloca %struct.M6__Foo_u64_, align 8
  %local_1__x = alloca i8, align 1
  %local_2__x = alloca i64, align 8
  %local_3__y = alloca %struct.M6__Foo_u64_, align 8
  %local_4 = alloca %struct.M6__Baz_u8.u64_, align 8
  %local_5__x = alloca i8, align 1
  %local_6__y = alloca %struct.M6__Foo_u64_, align 8
  %local_7 = alloca ptr, align 8
  %local_8__x = alloca ptr, align 8
  %local_9 = alloca i64, align 8
  store i8 123, ptr %local_1__x, align 1
  store i64 1992, ptr %local_2__x, align 4
  %fv.0 = load i64, ptr %local_2__x, align 4
  %insert_0 = insertvalue %struct.M6__Foo_u64_ undef, i64 %fv.0, 0
  store %struct.M6__Foo_u64_ %insert_0, ptr %local_3__y, align 4
  %fv.01 = load i8, ptr %local_1__x, align 1
  %fv.1 = load %struct.M6__Foo_u64_, ptr %local_3__y, align 4
  %insert_02 = insertvalue %struct.M6__Baz_u8.u64_ undef, i8 %fv.01, 0
  %insert_1 = insertvalue %struct.M6__Baz_u8.u64_ %insert_02, %struct.M6__Foo_u64_ %fv.1, 1
  store %struct.M6__Baz_u8.u64_ %insert_1, ptr %local_4, align 4
  %srcval = load %struct.M6__Baz_u8.u64_, ptr %local_4, align 4
  %ext_0 = extractvalue %struct.M6__Baz_u8.u64_ %srcval, 0
  %ext_1 = extractvalue %struct.M6__Baz_u8.u64_ %srcval, 1
  store i8 %ext_0, ptr %local_5__x, align 1
  store %struct.M6__Foo_u64_ %ext_1, ptr %local_6__y, align 4
  %load_store_tmp = load %struct.M6__Foo_u64_, ptr %local_6__y, align 4
  store %struct.M6__Foo_u64_ %load_store_tmp, ptr %local_0, align 4
  store ptr %local_0, ptr %local_7, align 8
  %tmp = load ptr, ptr %local_7, align 8
  %fld_ref = getelementptr inbounds %struct.M6__Foo_u64_, ptr %tmp, i32 0, i32 0
  store ptr %fld_ref, ptr %local_8__x, align 8
  %load_deref_store_tmp1 = load ptr, ptr %local_8__x, align 8
  %load_deref_store_tmp2 = load i64, ptr %load_deref_store_tmp1, align 4
  store i64 %load_deref_store_tmp2, ptr %local_9, align 4
  %rv.0 = load i8, ptr %local_5__x, align 1
  %rv.1 = load i64, ptr %local_9, align 4
  %insert_03 = insertvalue { i8, i64 } undef, i8 %rv.0, 0
  %insert_14 = insertvalue { i8, i64 } %insert_03, i64 %rv.1, 1
  ret { i8, i64 } %insert_14
}
