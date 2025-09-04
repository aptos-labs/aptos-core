// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// ==================================================================================
// Native object::exists_at

{%- for instance in object_instances %}
{%- set S = "'" ~ instance.suffix ~ "'" -%}
{%- set T = instance.name -%}
// ----------------------------------------------------------------------------------
// Native Object::exists_at for element type `{{instance.suffix}}`

procedure {:inline 1} $1_object_exists_at{{S}}(object: int) returns (res: bool) {
    res := $ResourceExists({{T}}_$memory, object);
}

{%- endfor %}


datatype $1_cmp_Ordering {
    $1_cmp_Ordering_Less(),
    $1_cmp_Ordering_Equal(),
    $1_cmp_Ordering_Greater()
}
function $IsValid'$1_cmp_Ordering_Less'(s: $1_cmp_Ordering): bool {
    true
}
function $IsValid'$1_cmp_Ordering_Equal'(s: $1_cmp_Ordering): bool {
    true
}
function $IsValid'$1_cmp_Ordering_Greater'(s: $1_cmp_Ordering): bool {
    true
}
function $IsValid'$1_cmp_Ordering'(s: $1_cmp_Ordering): bool {
    true
}
function {:inline} $IsEqual'$1_cmp_Ordering'(s1: $1_cmp_Ordering, s2: $1_cmp_Ordering): bool {
    s1 == s2
}

function $Arbitrary_value_of'$1_cmp_Ordering'(): $1_cmp_Ordering;

function {:inline} $1_cmp_$compare'bool'(s1: bool, s2: bool): $1_cmp_Ordering {
    if s1 == s2 then $1_cmp_Ordering_Equal()
    else if s1 == true then $1_cmp_Ordering_Greater()
    else 
        $1_cmp_Ordering_Less()
}

procedure {:inline 1} $1_cmp_compare'bool'(s1: bool, s2: bool) returns ($ret0: $1_cmp_Ordering)  {
    $ret0 := $1_cmp_$compare'bool'(s1, s2);
    return;
}

function {:inline} $1_cmp_$compare'signer'(s1: $signer, s2: $signer): $1_cmp_Ordering {
    if s1 == s2 then $1_cmp_Ordering_Equal()
    else if s1 is $signer && s2 is $permissioned_signer then $1_cmp_Ordering_Less()
    else if s1 is $permissioned_signer && s2 is $signer then $1_cmp_Ordering_Greater()
    else if s1 is $signer then
        $compare_int(s1 -> $addr, s2 -> $addr)
    else if s1 -> $addr == s2 -> $addr then
        $compare_int(s1 -> $permission_addr, s2 -> $permission_addr)
    else
        $compare_int(s1 -> $addr, s2 -> $addr)
}

procedure {:inline 1} $1_cmp_compare'signer'(s1: $signer, s2: $signer) returns ($ret0: $1_cmp_Ordering)  {
    $ret0 := $1_cmp_$compare'signer'(s1, s2);
    return;
}

function $compare_int(s1: int, s2: int): $1_cmp_Ordering {
    if s1 == s2 then $1_cmp_Ordering_Equal()
    else if s1 > s2 then $1_cmp_Ordering_Greater()
    else $1_cmp_Ordering_Less()
}

function {:inline} $1_cmp_$compare'num'(s1: int, s2: int): $1_cmp_Ordering {
    $compare_int(s1, s2)
}

procedure {:inline 1} $1_cmp_compare'num'(s1: int, s2: int) returns ($ret0: $1_cmp_Ordering)  {
    $ret0 := $compare_int(s1, s2);
    return;
}

function {:inline} $1_cmp_$compare'int'(s1: int, s2: int): $1_cmp_Ordering {
    $compare_int(s1, s2)
}

procedure {:inline 1} $1_cmp_compare'int'(s1: int, s2: int) returns ($ret0: $1_cmp_Ordering)  {
    $ret0 := $compare_int(s1, s2);
    return;
}

{%- for impl in bv_instances %}

function {:inline} $1_cmp_$compare'bv{{impl.base}}'(s1: bv{{impl.base}}, s2: bv{{impl.base}}): $1_cmp_Ordering {
    if s1 == s2 then $1_cmp_Ordering_Equal()
    else if $Gt'Bv{{impl.base}}'(s1,s2) then $1_cmp_Ordering_Greater()
    else $1_cmp_Ordering_Less()
}

procedure {:inline 1} $1_cmp_compare'bv{{impl.base}}'(s1: bv{{impl.base}}, s2: bv{{impl.base}}) returns ($ret0: $1_cmp_Ordering)  {
    $ret0 := $1_cmp_$compare'bv{{impl.base}}'(s1, s2);
    return;
}

{%- endfor %}


{%- for instance in cmp_int_instances -%}
{%- set S = instance.suffix  -%}
{%- set T = instance.name -%}


function {:inline} $1_cmp_$compare'{{S}}'(s1: {{T}}, s2: {{T}}): $1_cmp_Ordering {
    $compare_int(s1, s2)
}


procedure {:inline 1} $1_cmp_compare'{{S}}'(s1: {{T}}, s2: {{T}}) returns ($ret0: $1_cmp_Ordering)  {
    $ret0 := $compare_int(s1, s2);
    return;
}

{%- endfor %}

{%- for instance in cmp_vector_instances -%}
{%- set S = instance.suffix  -%}
{%- set T = instance.name -%}

    {% set concat_s = "/" ~ S ~"/" %}
    {% set rest_s = concat_s | trim_start_matches(pat="/vec'") | trim_end_matches(pat="'/") %}


    function {:inline} $1_cmp_$compare'{{S}}'(v1: {{T}}, v2: {{T}}): $1_cmp_Ordering {
        if $IsEqual'{{S}}'(v1, v2) then $1_cmp_Ordering_Equal()
        else if v1 -> l == 0 && v2 -> l != 0 then
            $1_cmp_Ordering_Less()
        else if v2 -> l == 0 && v1 -> l != 0 then
            $1_cmp_Ordering_Greater()
        else
            $compare_vec'{{S}}'(v1, v2)
    }

    procedure {:inline 1} $1_cmp_compare'{{S}}'(v1: {{T}}, v2: {{T}}) returns ($ret0: $1_cmp_Ordering) {
        $ret0 := $1_cmp_$compare'{{S}}'(v1, v2);
        return;
    }

    function $compare_vec'{{S}}'(v1: {{T}}, v2: {{T}}): $1_cmp_Ordering;
    axiom {:ctor "Vec"} (forall v1: {{T}}, v2: {{T}}, res: $1_cmp_Ordering ::
        (var res := $compare_vec'{{S}}'(v1, v2);
        if v1 -> l == 0 && v2 -> l != 0 then
            res == $1_cmp_Ordering_Less()
        else if v2 -> l == 0 && v1 -> l != 0 then
            res == $1_cmp_Ordering_Greater() 
        else if ReadVec(v1, 0) == ReadVec(v2, 0) then res == $compare_vec'{{S}}'(RemoveAtVec(v1, 0), RemoveAtVec(v2, 0))
        else res == $1_cmp_$compare'{{rest_s}}'(ReadVec(v1, 0), ReadVec(v2, 0))));
    
{%- endfor %}

{% for instance in cmp_table_instances -%}
{%- set S = instance.suffix  -%}
{%- set T = instance.name -%}

    function {:inline} $1_cmp_$compare'{{S}}'(v1: {{T}}, v2: {{T}}): $1_cmp_Ordering {
        $Arbitrary_value_of'$1_cmp_Ordering'()
    }

    procedure {:inline 1} $1_cmp_compare'{{S}}'(v1: {{T}}, v2: {{T}}) returns ($ret0: $1_cmp_Ordering) {
        $ret0 := $1_cmp_$compare'{{S}}'(v1, v2);
        return;
    }
    
{%- endfor %}


{% for instance in aggregator_v2_instances %}
{%- set S = instance.suffix  -%}
{%- set T = instance.name -%}

// ==================================================================================
// Intrinsic implementation of aggregator_v2 for element type `{{instance.suffix}}`


datatype $1_aggregator_v2_Aggregator'{{S}}' {
    $1_aggregator_v2_Aggregator'{{S}}'($value: {{T}}, $max_value: {{T}})
}
function {:inline} $Update'$1_aggregator_v2_Aggregator'{{S}}''_value(s: $1_aggregator_v2_Aggregator'{{S}}', x: {{T}}): $1_aggregator_v2_Aggregator'{{S}}' {
    $1_aggregator_v2_Aggregator'{{S}}'(x, s->$max_value)
}
function {:inline} $Update'$1_aggregator_v2_Aggregator'{{S}}''_max_value(s: $1_aggregator_v2_Aggregator'{{S}}', x: {{T}}): $1_aggregator_v2_Aggregator'{{S}}' {
    $1_aggregator_v2_Aggregator'{{S}}'(s->$value, x)
}
function $IsValid'$1_aggregator_v2_Aggregator'{{S}}''(s: $1_aggregator_v2_Aggregator'{{S}}'): bool {
    $IsValid'{{S}}'(s->$value)
      && $IsValid'{{S}}'(s->$max_value)
}
function {:inline} $IsEqual'$1_aggregator_v2_Aggregator'{{S}}''(s1: $1_aggregator_v2_Aggregator'{{S}}', s2: $1_aggregator_v2_Aggregator'{{S}}'): bool {
    $IsEqual'{{S}}'(s1->$value, s2->$value)
      && $IsEqual'{{S}}'(s1->$max_value, s2->$max_value)
}

procedure {:inline 1} $1_aggregator_v2_create_unbounded_aggregator'{{S}}'() returns (res: $1_aggregator_v2_Aggregator'{{S}}')
{
    {% if S == "u64" -%}
    res := $1_aggregator_v2_Aggregator'{{S}}'(0, $MAX_U64);
    {% elif S == "u128" -%}
    res := $1_aggregator_v2_Aggregator'{{S}}'(0, $MAX_U128);
    {% elif "#" in S -%}
    if (!$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec4(117, 49, 50, 56))) {
        call $ExecFailureAbort();
        return;
    }
    {% else -%}
        call $ExecFailureAbort();
        return;
    {% endif -%}
}


    procedure {:inline 1} $1_aggregator_v2_create_aggregator'{{S}}'($max_value: {{T}}) returns (res: $1_aggregator_v2_Aggregator'{{S}}')
    {
        {% if S == "u64" or S == "u128"  -%}
        res := $1_aggregator_v2_Aggregator'{{S}}'(0, $max_value);
        {% elif "#" in S -%}
        if (!$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec4(117, 49, 50, 56))) {
            call $ExecFailureAbort();
            return;
        }
        {% else -%}
        call $ExecFailureAbort();
        return;
        {% endif -%}
    }


    procedure {:inline 1} $1_aggregator_v2_try_add'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (res: bool, aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'))
    {
        {% if S == "u64" or S == "u128"  -%}
        if ($Dereference(aggregator)->$max_value < value + $Dereference(aggregator)->$value) {
            res := false;
            aggregator_updated:= aggregator;
        } else {
            res := true;
            aggregator_updated:= $UpdateMutation(aggregator, $1_aggregator_v2_Aggregator'{{S}}'(value + $Dereference(aggregator)->$value, $Dereference(aggregator)->$max_value));
        }
        {% elif "#" in S -%}
              if (!$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec4(117, 49, 50, 56))) {
                  call $ExecFailureAbort();
                  return;
              }
        {% else -%}
            call $ExecFailureAbort();
            return;
        {% endif -%}
    }

    procedure {:inline 1} $1_aggregator_v2_try_sub'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (res: bool, aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'))
    {
        {% if S == "u64" or S == "u128"  -%}
        if ($Dereference(aggregator)->$value < value) {
            res := false;
            aggregator_updated:= aggregator;
            return;
        } else {
            res := true;
            aggregator_updated:= $UpdateMutation(aggregator, $1_aggregator_v2_Aggregator'{{S}}'($Dereference(aggregator)->$value - value, $Dereference(aggregator)->$max_value));
            return;
        }
        {% elif "#" in S -%}
         if (!$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec4(117, 49, 50, 56))) {
             call $ExecFailureAbort();
             return;
         }
        {% else -%}
            call $ExecFailureAbort();
            return;
        {% endif -%}
    }

    procedure {:inline 1} $1_aggregator_v2_add'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'))
    {
       {% if S == "u64" or S == "u128"  -%}
       var try_result: bool;
       var try_aggregator: $Mutation $1_aggregator_v2_Aggregator'{{S}}';
       call try_result, try_aggregator := $1_aggregator_v2_try_add'{{S}}'(aggregator, value);
       if (!try_result) {
           call $ExecFailureAbort();
           return;
       }
       aggregator_updated := try_aggregator;
       return;
       {% elif "#" in S -%}
          var try_result: bool;
          var try_aggregator: $Mutation $1_aggregator_v2_Aggregator'{{S}}';
          call try_result, try_aggregator := $1_aggregator_v2_try_add'{{S}}'(aggregator, value);
          return;
       {% else -%}
        call $ExecFailureAbort();
        return;
       {% endif -%}
   }

   procedure {:inline 1} $1_aggregator_v2_sub'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'))
   {
       {% if S == "u64" or S == "u128"  -%}
          var try_result: bool;
          var try_aggregator: $Mutation $1_aggregator_v2_Aggregator'{{S}}';
          call try_result, try_aggregator := $1_aggregator_v2_try_sub'{{S}}'(aggregator, value);
          if (!try_result) {
              call $ExecFailureAbort();
              return;
          }
          aggregator_updated := try_aggregator;
         return;
       {% elif "#" in S -%}
          var try_result: bool;
          var try_aggregator: $Mutation $1_aggregator_v2_Aggregator'{{S}}';
          call try_result, try_aggregator := $1_aggregator_v2_try_add'{{S}}'(aggregator, value);
          return;
       {% else -%}
        call $ExecFailureAbort();
        return;
       {% endif -%}
   }

   procedure {:inline 1} $1_aggregator_v2_read'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}') returns (res: {{T}}) {
       {% if S == "u64" or S == "u128"  -%}
       res := aggregator->$value;
       {% elif "#" in S -%}
         if (!$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec4(117, 49, 50, 56))) {
             call $ExecFailureAbort();
             return;
         }
       {% else -%}
        call $ExecFailureAbort();
        return;
       {% endif -%}
   }

   procedure {:inline 1} $1_aggregator_v2_max_value'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}') returns (res: {{T}}) {
       {% if S == "u64" or S == "u128"  -%}
       res := aggregator->$max_value;
       {% elif "#" in S -%}
         if (!$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec4(117, 49, 50, 56))) {
             call $ExecFailureAbort();
             return;
         }
       {% else -%}
        call $ExecFailureAbort();
        return;
       {% endif -%}
   }

   procedure {:inline 1} $1_aggregator_v2_is_at_least_impl'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}', min_amount: {{T}}) returns (res: bool)
   {
       {% if S == "u64" or S == "u128"  -%}
          res := aggregator->$value >= min_amount;
          return;
       {% elif "#" in S -%}
         if (!$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($TypeName({{S}}_info), MakeVec4(117, 49, 50, 56))) {
             call $ExecFailureAbort();
             return;
         }
       {% else -%}
        call $ExecFailureAbort();
        return;
       {% endif -%}
   }

function {:inline} $1_aggregator_v2_spec_get_value'{{S}}'(s: $1_aggregator_v2_Aggregator'{{S}}'): {{T}} {
    s->$value
}

function {:inline} $1_aggregator_v2_spec_get_max_value'{{S}}'(s: $1_aggregator_v2_Aggregator'{{S}}'): {{T}} {
    s->$max_value
}

function {:inline} $1_aggregator_v2_$read'{{S}}'(s: $1_aggregator_v2_Aggregator'{{S}}'): {{T}} {
    s->$value
}

{% if S == "u64" or S == "u128" -%}
   function {:inline} $1_aggregator_v2_$is_at_least_impl'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}', min_amount: int): bool
   {
       aggregator->$value >= min_amount
   }
{% else -%}
   function $1_aggregator_v2_$is_at_least_impl'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}', min_amount: {{T}}): bool;
{% endif -%}

function {:inline} $1_cmp_$compare'$1_aggregator_v2_Aggregator'{{S}}''(s1: $1_aggregator_v2_Aggregator'{{S}}', s2: $1_aggregator_v2_Aggregator'{{S}}'): $1_cmp_Ordering {
    $Arbitrary_value_of'$1_cmp_Ordering'()
}

procedure {:inline 1} $1_cmp_compare'$1_aggregator_v2_Aggregator'{{S}}''(s1: $1_aggregator_v2_Aggregator'{{S}}', s2: $1_aggregator_v2_Aggregator'{{S}}') returns ($ret0: $1_cmp_Ordering)  {
    $ret0 := $1_cmp_$compare'$1_aggregator_v2_Aggregator'{{S}}''(s1, s2);
    return;
}

{%- endfor %}

// ==================================================================================
// Intrinsic implementation of aggregator and aggregator factory

datatype $1_aggregator_Aggregator {
    $1_aggregator_Aggregator($handle: int, $key: int, $limit: int, $val: int)
}
function {:inline} $Update'$1_aggregator_Aggregator'_handle(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(x, s->$key, s->$limit, s->$val)
}
function {:inline} $Update'$1_aggregator_Aggregator'_key(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(s->$handle, x, s->$limit, s->$val)
}
function {:inline} $Update'$1_aggregator_Aggregator'_limit(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(s->$handle, s->$key, x, s->$val)
}
function {:inline} $Update'$1_aggregator_Aggregator'_val(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(s->$handle, s->$key, s->$limit, x)
}
function $IsValid'$1_aggregator_Aggregator'(s: $1_aggregator_Aggregator): bool {
    $IsValid'address'(s->$handle)
      && $IsValid'address'(s->$key)
      && $IsValid'u128'(s->$limit)
      && $IsValid'u128'(s->$val)
}
function {:inline} $IsEqual'$1_aggregator_Aggregator'(s1: $1_aggregator_Aggregator, s2: $1_aggregator_Aggregator): bool {
    s1 == s2
}
function {:inline} $1_aggregator_spec_get_limit(s: $1_aggregator_Aggregator): int {
    s->$limit
}
function {:inline} $1_aggregator_limit(s: $1_aggregator_Aggregator): int {
    s->$limit
}
procedure {:inline 1} $1_aggregator_limit(s: $1_aggregator_Aggregator) returns (res: int) {
    res := s->$limit;
    return;
}
function {:inline} $1_aggregator_spec_get_handle(s: $1_aggregator_Aggregator): int {
    s->$handle
}
function {:inline} $1_aggregator_spec_get_key(s: $1_aggregator_Aggregator): int {
    s->$key
}
function {:inline} $1_aggregator_spec_get_val(s: $1_aggregator_Aggregator): int {
    s->$val
}

function $1_aggregator_spec_read(agg: $1_aggregator_Aggregator): int {
    $1_aggregator_spec_get_val(agg)
}

function $1_aggregator_spec_aggregator_set_val(agg: $1_aggregator_Aggregator, val: int): $1_aggregator_Aggregator {
    $Update'$1_aggregator_Aggregator'_val(agg, val)
}

function $1_aggregator_spec_aggregator_get_val(agg: $1_aggregator_Aggregator): int {
    $1_aggregator_spec_get_val(agg)
}

function $1_aggregator_factory_spec_new_aggregator(limit: int) : $1_aggregator_Aggregator;

axiom (forall limit: int :: {$1_aggregator_factory_spec_new_aggregator(limit)}
    (var agg := $1_aggregator_factory_spec_new_aggregator(limit);
     $1_aggregator_spec_get_limit(agg) == limit));

axiom (forall limit: int :: {$1_aggregator_factory_spec_new_aggregator(limit)}
     (var agg := $1_aggregator_factory_spec_new_aggregator(limit);
     $1_aggregator_spec_aggregator_get_val(agg) == 0));


function {:inline} $1_cmp_$compare'$1_aggregator_Aggregator'(s1: $1_aggregator_Aggregator, s2: $1_aggregator_Aggregator): $1_cmp_Ordering {
    $Arbitrary_value_of'$1_cmp_Ordering'()
}

procedure {:inline 1} $1_cmp_compare'$1_aggregator_Aggregator'(s1: $1_aggregator_Aggregator, s2: $1_aggregator_Aggregator) returns ($ret0: $1_cmp_Ordering)  {
    $ret0 := $1_cmp_$compare'$1_aggregator_Aggregator'(s1, s2);
    return;
}


// ==================================================================================
// Native for function_info

procedure $1_function_info_is_identifier(s: Vec int) returns (res: bool);
