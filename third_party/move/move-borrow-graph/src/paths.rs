// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub type PathSlice<Lbl> = [Lbl];
pub type Path<Lbl> = Vec<Lbl>;

pub fn leq<Lbl: Eq>(lhs: &PathSlice<Lbl>, rhs: &PathSlice<Lbl>) -> bool {
    lhs.len() <= rhs.len() && lhs.iter().zip(rhs).all(|(l, r)| l == r)
}

pub fn factor<Lbl: Eq>(lhs: &PathSlice<Lbl>, mut rhs: Path<Lbl>) -> (Path<Lbl>, Path<Lbl>) {
    assert!(leq(lhs, &rhs));
    let suffix = rhs.split_off(lhs.len());
    (rhs, suffix)
}

pub fn append<Lbl: Clone>(lhs: &PathSlice<Lbl>, rhs: &PathSlice<Lbl>) -> Path<Lbl> {
    let mut path: Path<Lbl> = lhs.into();
    path.append(&mut rhs.to_owned());
    path
}
