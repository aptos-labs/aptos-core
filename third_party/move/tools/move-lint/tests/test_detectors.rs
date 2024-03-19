use std::path::PathBuf;

#[test]
fn test_modules() {
    let path = PathBuf::from("tests").join("cases").join("exceed_params");
    move_lint::move_lint(path);
}
