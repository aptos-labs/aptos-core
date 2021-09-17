// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::unit_tests::testutils::compile_script_string;
use move_binary_format::file_format::Bytecode::*;

#[test]
fn compile_if_else_with_fallthrough() {
    let code = String::from(
        "
        main() {
            let x: u64;
        label b0:
            jump_if (42 > 0) b2;
        label b1:
            x = 1;
        label b2:
            return;
        }
        ",
    );

    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrTrue(_)), 1);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 0);
}

#[test]
fn compile_if_else_with_jumps() {
    let code = String::from(
        "
        main() {
            let x: u64;
            let y: u64;
        label b0:
            jump_if (42 > 0) b2;
        label b1:
            x = 1;
            jump b3;
        label b2:
            y = 1;
            jump b3;
        label b3:
            return;
        }
        ",
    );

    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrTrue(_)), 1);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 2);
}

#[test]
fn compile_nested_if_else() {
    let code = String::from(
        "
        main() {
            let x: u64;
        label b0:
            jump_if (42 > 0) b2;
        label b1:
            x = 1;
            jump exit;
        label b2:
            jump_if (5 > 10) b4;
        label b3:
            x = 2;
            jump exit;
        label b4:
            x = 3;
            jump exit;
        label exit:
            return;
        }
        ",
    );

    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrTrue(_)), 2);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 3);
}

#[test]
fn compile_if_else_with_if_return() {
    let code = String::from(
        "
        main() {
            let x: u64;
        label b0:
            jump_if (42 > 0) b2;
        label b1:
            x = 1;
            jump b3;
        label b2:
            return;
        label b3:
            return;
        }
        ",
    );

    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrTrue(_)), 1);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 1);
    assert_eq!(instr_count!(compiled_script, Ret), 2);
}

#[test]
fn compile_if_else_with_else_return() {
    let code = String::from(
        "
        main() {
            let x: u64;
        label b0:
            jump_if (42 > 0) b2;
        label b1:
            return;
        label b2:
            x = 1;
            jump b3;
        label b3:
            return;
        }
        ",
    );

    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrTrue(_)), 1);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 1);
    assert_eq!(instr_count!(compiled_script, Ret), 2);
}

#[test]
fn compile_if_else_with_two_returns() {
    let code = String::from(
        "
        main() {
        label b0:
            jump_if (42 > 0) b2;
        label b1:
            return;
        label b2:
            return;
        }
        ",
    );

    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrTrue(_)), 1);
    assert_eq!(instr_count!(compiled_script, Ret), 2);
}

#[test]
fn compile_while() {
    let code = String::from(
        "
        main() {
            let x: u64;
        label b0:
            x = 0;
        label while:
            jump_if_false (copy(x) < 5) while_cont;
        label while_b0:
            x = copy(x) + 1;
            jump while;
        label while_cont:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrFalse(_)), 1);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 1);
}

#[test]
fn compile_while_return() {
    let code = String::from(
        "
        main() {
        label while:
            jump_if_false (42 > 0) while_cont;
        label while_b0:
            return;
        label while_cont:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrFalse(_)), 1);
    assert_eq!(instr_count!(compiled_script, Ret), 2);
}

#[test]
fn compile_nested_while() {
    let code = String::from(
        "
        main() {
            let x: u64;
            let y: u64;
        label b0:
            x = 0;
        label outer_while:
            jump_if_false (copy(x) < 5) outer_while_cont;
        label outer_while_b0:
            x = move(x) + 1;
            y = 0;
        label inner_while:
            jump_if_false (copy(y) < 5) inner_while_cont;
        label inner_while_b0:
            y = move(y) + 1;
            jump inner_while;
        label inner_while_cont:
            jump outer_while;
        label outer_while_cont:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrFalse(_)), 2);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 2);
}

#[test]
fn compile_while_break() {
    let code = String::from(
        "
        main() {
        label while:
            jump_if_false (true) while_cont;
        label while_b0:
            jump while_cont;
        label while_cont:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrFalse(_)), 1);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 1);
}

#[test]
fn compile_while_continue() {
    let code = String::from(
        "
        main() {
        label while:
            jump_if_false (false) while_cont;
        label while_b0:
            jump while;
        label while_cont:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, BrFalse(_)), 1);
    assert_eq!(instr_count!(compiled_script, Branch(_)), 1);
}

#[test]
fn compile_loop_empty() {
    let code = String::from(
        "
        main() {
        label loop:
            jump loop;
        label exit:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, Branch(_)), 1);
}

#[test]
fn compile_loop_nested_break() {
    let code = String::from(
        "
        main() {
        label outer_loop:
        label inner_loop:
            jump inner_loop_cont;
            jump inner_loop;
        label inner_loop_cont:
            jump outer_loop_cont;
            jump outer_loop;
        label outer_loop_cont:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, Branch(_)), 4);
}

#[test]
fn compile_loop_break_continue() {
    let code = String::from(
        "
        main() {
            let x: u64;
            let y: u64;
        label b0:
            x = 0;
            y = 0;
        label loop:
            x = move(x) + 1;
            jump_if (copy(x) >= 10) loop_b2;
        label loop_b0:
            jump_if (copy(x) % 2 == 0) loop_b3;
        label loop_b1:
            y = move(y) + copy(x);
            jump loop;
        label loop_b2:
            jump loop_cont;
        label loop_b3:
            jump loop;
        label loop_cont:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, Branch(_)), 3);
    assert_eq!(instr_count!(compiled_script, BrTrue(_)), 2);
}

#[test]
fn compile_loop_return() {
    let code = String::from(
        "
        main() {
        label outer_loop:
        label inner_loop:
            return;
            jump inner_loop;
        label inner_loop_cont:
            return;
            jump outer_loop;
        label outer_loop_cont:
            return;
        }
        ",
    );
    let compiled_script_res = compile_script_string(&code);
    let compiled_script = compiled_script_res.unwrap();
    assert_eq!(instr_count!(compiled_script, Branch(_)), 2);
    assert_eq!(instr_count!(compiled_script, Ret), 3);
}
