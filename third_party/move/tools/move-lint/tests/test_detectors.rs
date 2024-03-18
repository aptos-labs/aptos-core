use std::path::PathBuf;

#[test]
fn test_modules() {
    let path = PathBuf::from("tests").join("cases").join("self_assignment");
    move_lint::move_lint(path);
}
