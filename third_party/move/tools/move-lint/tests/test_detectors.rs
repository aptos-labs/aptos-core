use std::path::PathBuf;

#[test]
fn test_modules() {
    let path = PathBuf::from("tests")
        .join("cases")
        .join("unused_borrow_global_mut");
    move_lint::move_lint(path);
}
