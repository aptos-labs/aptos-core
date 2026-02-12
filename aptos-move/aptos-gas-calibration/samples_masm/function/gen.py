import os

template = """
    // Calling function with {num_args} args & {num_locals} locals.
    {name}({params}) {{
        {ls}
    label b0:
        return;
    }}

    public calibrate_call_{name}_impl(n: u64) {{
        let i: u64;
    label entry:
        i = 0;
    label loop_start:
        jump_if_false (copy(i) < copy(n)) loop_end;
        i = move(i) + 1;

        Self.{name}({args});

        jump loop_start;
    label loop_end:
        return;
    }}

    public entry calibrate_call_{name}_x100() {{
    label b0:
        Self.calibrate_call_{name}_impl(10);
        return;       
    }}

    public entry calibrate_{name}_x500() {{
    label b0:
        Self.calibrate_call_{name}_impl(50);
        return;       
    }}

    public entry calibrate_{name}_x1000() {{
    label b0:
        Self.calibrate_call_{name}_impl(100);
        return;       
    }}
"""

def gen_calibration_sample(num_args, num_locals):
    name = "a{}_l{}".format(num_args, num_locals)
    ls = '\n        '.join(['let l{}: u64;'.format(i) for i in range(num_locals)])
    params = ', '.join(['a{}: u64'.format(i) for i in range(num_args)])
    args = ', '.join(['0'] * num_args)
    return template.format(num_args = num_args, num_locals = num_locals, name = name, params = params, args = args, ls = ls)

with open(os.path.dirname(__file__) + "/call.mvir", "w") as f:
    f.write("// !!! GENERATED FILE -- DO NOT EDIT MANUALLY !!!\n")
    f.write("module 0xcafe.Call {")

    f.write(gen_calibration_sample(0, 0))

    f.write(gen_calibration_sample(4, 0))
    f.write(gen_calibration_sample(16, 0))
    f.write(gen_calibration_sample(64, 0))
    
    f.write(gen_calibration_sample(0, 4))
    f.write(gen_calibration_sample(0, 16))
    f.write(gen_calibration_sample(0, 64))
    f.write("}")
