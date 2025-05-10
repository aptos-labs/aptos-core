use aptos_drop_helper::ArcAsyncDrop;
use aptos_experimental_layered_map::{Key, MapLayer, Value};

pub struct LRULayer<K: ArcAsyncDrop, V: ArcAsyncDrop> {
    inner: MapLayer<K, V>,
}

pub struct LayeredLRU<K: ArcAsyncDrop, V: ArcAsyncDrop> {
    base_layer: LRULayer<K, V>,
    top_layer: LRULayer<K, V>,
}

impl<K, V> LayeredLRU<K, V>
where
    K: ArcAsyncDrop + Key,
    V: ArcAsyncDrop + Value,
{
    /// Create a new layer with added items. Returns items evicted from the LRU.
    pub fn new_layer(&self, items: &[(K, V)]) -> (LRULayer<K, V>, Vec<(K, V)>) {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
