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
