use std::path::PathBuf;


#[test]
fn test_modules() {
    let path = PathBuf::from("tests").join("cases").join("borrow_deref_ref");
    aptos_lint::aptos_lint(path);
}
