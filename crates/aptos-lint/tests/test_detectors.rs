use std::path::PathBuf;


fn run_linter(path: PathBuf) {
    let context = aptos_lint::aptos_lint(path).unwrap();
    context.emit_diagnostics();
}

#[test]
fn test_modules() {
    run_linter(PathBuf::from("tests").join("cases").join("bool_comparision"));
    run_linter(PathBuf::from("tests").join("cases").join("double_bool_comparison"));
    run_linter(PathBuf::from("tests").join("cases").join("ifs_same_cond"));
    run_linter(PathBuf::from("tests").join("cases").join("multiplication_before_division"));
    run_linter(PathBuf::from("tests").join("cases").join("shift_overflow"));
    run_linter(PathBuf::from("tests").join("cases").join("unnecessary_type_conversion"));
    run_linter(PathBuf::from("tests").join("cases").join("unused_private_function"));
}
