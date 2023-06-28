; ModuleID = '0x100__Country'
source_filename = "<unknown>"

%struct.Country__Country = type { i8, i64, %struct.Country__Dunno, i8 }
%struct.Country__Dunno = type { i64, i8 }

declare i32 @memcmp(ptr, ptr, i64)

define i8 @Country__dropit(%struct.Country__Country %0) {
entry:
  %local_0 = alloca %struct.Country__Country, align 8
  %local_1 = alloca %struct.Country__Country, align 8
  %local_2__id = alloca i8, align 1
  %local_3__population = alloca i64, align 8
  %local_4__phony = alloca %struct.Country__Dunno, align 8
  store %struct.Country__Country %0, ptr %local_0, align 4
  %srcval = load %struct.Country__Country, ptr %local_0, align 4
  %ext_0 = extractvalue %struct.Country__Country %srcval, 0
  %ext_1 = extractvalue %struct.Country__Country %srcval, 1
  %ext_2 = extractvalue %struct.Country__Country %srcval, 2
  store i8 %ext_0, ptr %local_2__id, align 1
  store i64 %ext_1, ptr %local_3__population, align 4
  store %struct.Country__Dunno %ext_2, ptr %local_4__phony, align 4
  %retval = load i8, ptr %local_2__id, align 1
  ret i8 %retval
}

define i8 @Country__get_id(ptr %0) {
entry:
  %local_0 = alloca ptr, align 8
  %local_1 = alloca ptr, align 8
  %local_2__id = alloca ptr, align 8
  %local_3 = alloca i8, align 1
  store ptr %0, ptr %local_0, align 8
  %load_store_tmp = load ptr, ptr %local_0, align 8
  store ptr %load_store_tmp, ptr %local_1, align 8
  %tmp = load ptr, ptr %local_1, align 8
  %fld_ref = getelementptr inbounds %struct.Country__Country, ptr %tmp, i32 0, i32 0
  store ptr %fld_ref, ptr %local_2__id, align 8
  %load_deref_store_tmp1 = load ptr, ptr %local_2__id, align 8
  %load_deref_store_tmp2 = load i8, ptr %load_deref_store_tmp1, align 1
  store i8 %load_deref_store_tmp2, ptr %local_3, align 1
  %retval = load i8, ptr %local_3, align 1
  ret i8 %retval
}

define i64 @Country__get_phony_x(%struct.Country__Country %0) {
entry:
  %local_0 = alloca %struct.Country__Country, align 8
  %local_1 = alloca ptr, align 8
  %local_2__phony = alloca ptr, align 8
  %local_3__x = alloca ptr, align 8
  %local_4 = alloca i64, align 8
  store %struct.Country__Country %0, ptr %local_0, align 4
  store ptr %local_0, ptr %local_1, align 8
  %tmp = load ptr, ptr %local_1, align 8
  %fld_ref = getelementptr inbounds %struct.Country__Country, ptr %tmp, i32 0, i32 2
  store ptr %fld_ref, ptr %local_2__phony, align 8
  %tmp1 = load ptr, ptr %local_2__phony, align 8
  %fld_ref2 = getelementptr inbounds %struct.Country__Dunno, ptr %tmp1, i32 0, i32 0
  store ptr %fld_ref2, ptr %local_3__x, align 8
  %load_deref_store_tmp1 = load ptr, ptr %local_3__x, align 8
  %load_deref_store_tmp2 = load i64, ptr %load_deref_store_tmp1, align 4
  store i64 %load_deref_store_tmp2, ptr %local_4, align 4
  %retval = load i64, ptr %local_4, align 4
  ret i64 %retval
}

define i64 @Country__get_pop(%struct.Country__Country %0) {
entry:
  %local_0 = alloca %struct.Country__Country, align 8
  %local_1 = alloca ptr, align 8
  %local_2__population = alloca ptr, align 8
  %local_3 = alloca i64, align 8
  store %struct.Country__Country %0, ptr %local_0, align 4
  store ptr %local_0, ptr %local_1, align 8
  %tmp = load ptr, ptr %local_1, align 8
  %fld_ref = getelementptr inbounds %struct.Country__Country, ptr %tmp, i32 0, i32 1
  store ptr %fld_ref, ptr %local_2__population, align 8
  %load_deref_store_tmp1 = load ptr, ptr %local_2__population, align 8
  %load_deref_store_tmp2 = load i64, ptr %load_deref_store_tmp1, align 4
  store i64 %load_deref_store_tmp2, ptr %local_3, align 4
  %retval = load i64, ptr %local_3, align 4
  ret i64 %retval
}

define %struct.Country__Country @Country__new_country(i8 %0, i64 %1) {
entry:
  %local_0 = alloca i8, align 1
  %local_1 = alloca i64, align 8
  %local_2__id = alloca i8, align 1
  %local_3__population = alloca i64, align 8
  %local_4__x = alloca i64, align 8
  %local_5__phony = alloca %struct.Country__Dunno, align 8
  %local_6 = alloca %struct.Country__Country, align 8
  store i8 %0, ptr %local_0, align 1
  store i64 %1, ptr %local_1, align 4
  %load_store_tmp = load i8, ptr %local_0, align 1
  store i8 %load_store_tmp, ptr %local_2__id, align 1
  %load_store_tmp1 = load i64, ptr %local_1, align 4
  store i64 %load_store_tmp1, ptr %local_3__population, align 4
  store i64 32, ptr %local_4__x, align 4
  %fv.0 = load i64, ptr %local_4__x, align 4
  %insert_0 = insertvalue %struct.Country__Dunno undef, i64 %fv.0, 0
  store %struct.Country__Dunno %insert_0, ptr %local_5__phony, align 4
  %fv.02 = load i8, ptr %local_2__id, align 1
  %fv.1 = load i64, ptr %local_3__population, align 4
  %fv.2 = load %struct.Country__Dunno, ptr %local_5__phony, align 4
  %insert_03 = insertvalue %struct.Country__Country undef, i8 %fv.02, 0
  %insert_1 = insertvalue %struct.Country__Country %insert_03, i64 %fv.1, 1
  %insert_2 = insertvalue %struct.Country__Country %insert_1, %struct.Country__Dunno %fv.2, 2
  store %struct.Country__Country %insert_2, ptr %local_6, align 4
  %retval = load %struct.Country__Country, ptr %local_6, align 4
  ret %struct.Country__Country %retval
}

define void @Country__set_id(ptr %0, i8 %1) {
entry:
  %local_0 = alloca ptr, align 8
  %local_1 = alloca i8, align 1
  %local_2 = alloca i8, align 1
  %local_3 = alloca ptr, align 8
  %local_4__id = alloca ptr, align 8
  store ptr %0, ptr %local_0, align 8
  store i8 %1, ptr %local_1, align 1
  %load_store_tmp = load i8, ptr %local_1, align 1
  store i8 %load_store_tmp, ptr %local_2, align 1
  %load_store_tmp1 = load ptr, ptr %local_0, align 8
  store ptr %load_store_tmp1, ptr %local_3, align 8
  %tmp = load ptr, ptr %local_3, align 8
  %fld_ref = getelementptr inbounds %struct.Country__Country, ptr %tmp, i32 0, i32 0
  store ptr %fld_ref, ptr %local_4__id, align 8
  %load_store_ref_src = load i8, ptr %local_2, align 1
  %load_store_ref_dst_ptr = load ptr, ptr %local_4__id, align 8
  store i8 %load_store_ref_src, ptr %load_store_ref_dst_ptr, align 1
  ret void
}
