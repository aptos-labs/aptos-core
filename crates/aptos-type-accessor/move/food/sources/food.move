module addr::food {
    use aptos_std::simple_map::SimpleMap;
    use aptos_std::table::Table;
    use std::string::String;

    struct Color has store {
        red: u8,
        blue: u8,
        green: u8,
    }

    struct Fruit has store {
        name: String,
        color: Color,
    }

    struct Buyer has store {
        name: String,
        address: address,
    }

    struct FruitManager has key {
        // The key is just an incrementing counter. This tracks all the fruit we have.
        fruit_inventory: Table<u64, Fruit>,

        // A map from fruit name to price.
        prices: SimpleMap<String, u64>,

        // A list of addresses authorized to buy the fruit.
        authorized_buyers: vector<Buyer>,

        // The last time a piece of fruit was sold.
        last_sale_time: u64,
    }
}
