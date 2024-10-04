// Copyright © Aptos Foundation
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




{%- for instance in aggregator_v2_instances %}
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

{% if S == "u64" -%}

procedure {:inline 1} $1_aggregator_v2_create_unbounded_aggregator'u64'() returns (res: $1_aggregator_v2_Aggregator'u64')
{
   res := $1_aggregator_v2_Aggregator'{{S}}'(0, $MAX_U64);
}

{% elif S == "u128" -%}

procedure {:inline 1} $1_aggregator_v2_create_unbounded_aggregator'u128'() returns (res: $1_aggregator_v2_Aggregator'u128')
{
   res := $1_aggregator_v2_Aggregator'{{S}}'(0, $MAX_U128);
}

{% endif -%}


{% if S == "u64" or S == "u128"  -%}

procedure {:inline 1} $1_aggregator_v2_create_aggregator'{{S}}'($max_value: int) returns (res: $1_aggregator_v2_Aggregator'{{S}}')
{
    res := $1_aggregator_v2_Aggregator'{{S}}'(0, $max_value);
}


procedure {:inline 1} $1_aggregator_v2_try_add'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: int) returns (res: bool, aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'))
 {
    if ($Dereference(aggregator)->$max_value < value + $Dereference(aggregator)->$value) {
        res := false;
        aggregator_updated:= aggregator;
    } else {
        res := true;
        aggregator_updated:= $UpdateMutation(aggregator, $1_aggregator_v2_Aggregator'{{S}}'(value + $Dereference(aggregator)->$value, $Dereference(aggregator)->$max_value));
    }
}

procedure {:inline 1} $1_aggregator_v2_try_sub'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: int) returns (res: bool, aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'))
{
    if ($Dereference(aggregator)->$value < value) {
        res := false;
        aggregator_updated:= aggregator;
        return;
    } else {
        res := true;
        aggregator_updated:= $UpdateMutation(aggregator, $1_aggregator_v2_Aggregator'{{S}}'($Dereference(aggregator)->$value - value, $Dereference(aggregator)->$max_value));
        return;
    }
}

   procedure {:inline 1} $1_aggregator_v2_add'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: int) returns (aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'))
    {
       var try_result: bool;
       var try_aggregator: $Mutation $1_aggregator_v2_Aggregator'{{S}}';
       call try_result, try_aggregator := $1_aggregator_v2_try_add'{{S}}'(aggregator, value);
       if (!try_result) {
           call $ExecFailureAbort();
           return;
       }
       aggregator_updated := try_aggregator;
       return;
   }

   procedure {:inline 1} $1_aggregator_v2_sub'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: int) returns (aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'))
   {
          var try_result: bool;
          var try_aggregator: $Mutation $1_aggregator_v2_Aggregator'{{S}}';
          call try_result, try_aggregator := $1_aggregator_v2_try_sub'{{S}}'(aggregator, value);
          if (!try_result) {
              call $ExecFailureAbort();
              return;
          }
          aggregator_updated := try_aggregator;
         return;
   }

   procedure {:inline 1} $1_aggregator_v2_read'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}') returns (res: {{T}}) {
       res := aggregator->$value;
   }

   procedure {:inline 1} $1_aggregator_v2_is_at_least_impl'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}', min_amount: int) returns (res: bool)
   {
          res := aggregator->$value >= min_amount;
          return;
   }

   function {:inline} $1_aggregator_v2_$is_at_least_impl'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}', min_amount: int): bool
   {
       aggregator->$value >= min_amount
   }


{% elif "#" in S -%}
   procedure {:inline 1} $1_aggregator_v2_create_aggregator'{{S}}'($max_value: {{T}}) returns (res: $1_aggregator_v2_Aggregator'{{S}}') {
      if (!$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec4(117, 49, 50, 56))) {
          call $ExecFailureAbort();
          return;
      }
      res := $1_aggregator_v2_$create_aggregator'{{S}}'($max_value);
   }

   function $1_aggregator_v2_$create_aggregator'{{S}}'($max_value: {{T}}) : $1_aggregator_v2_Aggregator'{{S}}';

   procedure {:inline 1} $1_aggregator_v2_create_unbounded_aggregator'{{S}}'() returns (res: $1_aggregator_v2_Aggregator'{{S}}') {
      if (!$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec4(117, 49, 50, 56))) {
          call $ExecFailureAbort();
          return;
      }
      res := $1_aggregator_v2_$create_unbound_aggregator'{{S}}'();
   }

   function $1_aggregator_v2_$create_unbound_aggregator'{{S}}'() : $1_aggregator_v2_Aggregator'{{S}}';

   procedure {:inline 1} $1_aggregator_v2_try_add'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (res: bool, aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}')) {
         if (!$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec4(117, 49, 50, 56))) {
             call $ExecFailureAbort();
             return;
         }
   }

   procedure {:inline 1} $1_aggregator_v2_try_sub'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (res: bool, aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}')) {
         if (!$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec4(117, 49, 50, 56))) {
             call $ExecFailureAbort();
             return;
         }
   }

   procedure {:inline 1} $1_aggregator_v2_add'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}')) {
          var try_result: bool;
          var try_aggregator: $Mutation $1_aggregator_v2_Aggregator'{{S}}';
          call try_result, try_aggregator := $1_aggregator_v2_try_add'{{S}}'(aggregator, value);
          return;
   }

   procedure {:inline 1} $1_aggregator_v2_sub'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}')) {
          var try_result: bool;
          var try_aggregator: $Mutation $1_aggregator_v2_Aggregator'{{S}}';
          call try_result, try_aggregator := $1_aggregator_v2_try_sub'{{S}}'(aggregator, value);
          return;
   }

   procedure {:inline 1} $1_aggregator_v2_is_at_least_impl'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}', min_amount: {{T}}) returns (res: bool) {
         if (!$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec4(117, 49, 50, 56))) {
             call $ExecFailureAbort();
             return;
         }
   }

   procedure {:inline 1} $1_aggregator_v2_read'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}') returns (res: {{T}}) {
         if (!$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec3(117, 54, 52)) && !$IsEqual'vec'u8''($1_string_String($TypeName({{S}}_info))->$bytes, MakeVec4(117, 49, 50, 56))) {
             call $ExecFailureAbort();
             return;
         }
         res := aggregator->$value;
   }

{% else -%}
   procedure {:inline 1} $1_aggregator_v2_create_aggregator'{{S}}'($max_value: {{T}}) returns (res: $1_aggregator_v2_Aggregator'{{S}}') {
        call $ExecFailureAbort();
        return;
   }

   procedure {:inline 1} $1_aggregator_v2_create_unbounded_aggregator'{{S}}'() returns (res: $1_aggregator_v2_Aggregator'{{S}}') {
        call $ExecFailureAbort();
        return;
   }

   procedure {:inline 1} $1_aggregator_v2_try_add'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (res: bool, aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}')) {
        call $ExecFailureAbort();
        return;
   }

   procedure {:inline 1} $1_aggregator_v2_try_sub'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (res: bool, aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}')) {
        call $ExecFailureAbort();
        return;
   }

   procedure {:inline 1} $1_aggregator_v2_add'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}')) {
        call $ExecFailureAbort();
        return;
   }

   procedure {:inline 1} $1_aggregator_v2_sub'{{S}}'(aggregator: $Mutation ($1_aggregator_v2_Aggregator'{{S}}'), value: {{T}}) returns (aggregator_updated: $Mutation ($1_aggregator_v2_Aggregator'{{S}}')) {
        call $ExecFailureAbort();
        return;
   }

   procedure {:inline 1} $1_aggregator_v2_is_at_least_impl'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}', min_amount: {{T}}) returns (res: bool)  {
      call $ExecFailureAbort();
      return;
   }

   procedure {:inline 1} $1_aggregator_v2_read'{{S}}'(aggregator: $1_aggregator_v2_Aggregator'{{S}}') returns (res: {{T}}) {
       call $ExecFailureAbort();
       return;
   }


{% endif %}

function {:inline} $1_aggregator_v2_spec_get_value'{{S}}'(s: $1_aggregator_v2_Aggregator'{{S}}'): {{T}} {
    s->$value
}

function {:inline} $1_aggregator_v2_spec_get_max_value'{{S}}'(s: $1_aggregator_v2_Aggregator'{{S}}'): {{T}} {
    s->$max_value
}

function {:inline} $1_aggregator_v2_$read'{{S}}'(s: $1_aggregator_v2_Aggregator'{{S}}'): {{T}} {
    s -> $value
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

// ==================================================================================
// Native for function_info

procedure $1_function_info_is_identifier(s: Vec int) returns (res: bool);
