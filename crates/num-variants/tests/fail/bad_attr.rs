use aptos_num_variants::NumVariants;

#[derive(NumVariants)]
#[num_variants = 123] // invalid (must be string)
enum BadAttr {
    A,
    B,
}

fn main() {}