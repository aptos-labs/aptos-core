use aptos_num_variants::NumVariants;

#[derive(NumVariants)]
struct NotEnum {
    x: u32,
}

fn main() {}