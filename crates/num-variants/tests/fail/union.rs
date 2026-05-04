use aptos_num_variants::NumVariants;

#[derive(NumVariants)]
union MyUnion {
    a: u32,
    b: f32,
}

fn main() {}