use aptos_experimental_layered_map::{tests::HashCollide, MapLayer};

fn test_basic_stuff() {
    let persisted = MapLayer::<HashCollide, String>::new_family("test_basic");
    let layer0 = persisted.clone();
    let map0 = layer0.view_layers_after(&persisted);
    println!("map0: {:?}", map0.iter().collect::<Vec<_>>());

    let kvs1 = vec![
        (HashCollide(0), "a".to_string()),
        (HashCollide(1), "bb".to_string()),
        (HashCollide(2), "ccc".to_string()),
        (HashCollide(3), "dddd".to_string()),
    ];
    let layer1 = map0.new_layer(&kvs1);
    let map10 = layer1.view_layers_after(&layer0);
    println!("map10: {:?}", map10.iter().collect::<Vec<_>>());

    let kvs2 = vec![
        (HashCollide(0), "aa".to_string()),
        (HashCollide(1), "bbbb".to_string()),
        (HashCollide(4), "eeeee".to_string()),
        (HashCollide(5), "ffffff".to_string()),
    ];
    let layer2 = map10.new_layer(&kvs2);
    let map21 = layer2.view_layers_after(&layer1);
    println!("map21: {:?}", map21.iter().collect::<Vec<_>>());

    let map20 = layer2.view_layers_after(&layer0);
    println!("map20: {:?}", map20.iter().collect::<Vec<_>>());

    println!("------------");
    println!("{:?}", map10.get(&HashCollide(0)));
    println!("{:?}", map21.get(&HashCollide(0)));
    println!("{:?}", map20.get(&HashCollide(0)));
    println!("------------");
    println!("{:?}", map10.get(&HashCollide(2)));
    println!("{:?}", map21.get(&HashCollide(2)));
    println!("{:?}", map20.get(&HashCollide(2)));
    println!("------------");

    let kvs2_ = vec![(HashCollide(0), "aaaaaaaaaaaaaaaaaaa".to_string())];
    let layer2_ = map10.new_layer(&kvs2_);
    let map21_ = layer2_.view_layers_after(&layer1);
    println!("map21_: {:?}", map21_.iter().collect::<Vec<_>>());

    let map20_ = layer2_.view_layers_after(&layer0);
    println!("map20_: {:?}", map20_.iter().collect::<Vec<_>>());
}

fn main() {
    test_basic_stuff();
}
