use aptos_experimental_layered_map::MapLayer;
use std::collections::HashMap;

pub struct InMemHotState {
    state: MapLayer<u64, String>,
}

impl InMemHotState {
    pub fn new(use_case: &'static str) -> Self {
        Self {
            state: MapLayer::new_family(use_case),
        }
    }

    pub fn update(&self, updates: &HashMap<u64, String>) -> Self {
        println!("updates: {updates:?}");
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::InMemHotState;
    use std::collections::HashMap;

    #[test]
    fn test_basic() {
        let state = InMemHotState::new("test");

        let mut updates = HashMap::new();
        updates.insert(1, "a".to_string());
        updates.insert(2, "bb".to_string());
        updates.insert(3, "ccc".to_string());

        let new_state = state.update(&updates);
    }
}
