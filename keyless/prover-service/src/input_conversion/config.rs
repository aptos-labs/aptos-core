 use std::collections::BTreeMap;
 use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize)]
pub struct CircuitConfig {
    pub global_input_max_lengths : BTreeMap<String, usize>,
    pub field_check_inputs: FieldCheckInputConfigs,
}


#[derive(Serialize, Deserialize)]
pub struct FieldCheckInputConfigs(Vec<FieldCheckInputConfig>);

impl FieldCheckInputConfigs {
    pub fn max_name_length(&self, name: &str) -> Option<usize> {
        Some(
            self.0
            .iter()
            .find(|field_config| &field_config.circuit_input_signal_prefix == name)?
            .max_name_length
            )
    }
    pub fn max_value_length(&self, name: &str) -> Option<usize> {
        Some(
            self.0
            .iter()
            .find(|field_config| &field_config.circuit_input_signal_prefix == name)?
            .max_value_length
            )
    }
    pub fn max_whole_field_length(&self, name: &str) -> Option<usize> {
        Some(
            self.0
            .iter()
            .find(|field_config| &field_config.circuit_input_signal_prefix == name)?
            .max_whole_field_length
            )
    }
}


impl<'a> IntoIterator for &'a FieldCheckInputConfigs {
    type Item = <&'a Vec<FieldCheckInputConfig> as IntoIterator>::Item;

    type IntoIter = <&'a Vec<FieldCheckInputConfig> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).iter()
    }
}




#[derive(Serialize, Deserialize)]
pub struct FieldCheckInputConfig {
    pub circuit_input_signal_prefix: String,
    pub jwt_key: Key,
    pub has_value_inputs: bool,
    pub max_name_length: usize,
    pub max_value_length: usize,
    pub max_whole_field_length: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Key {
    Fixed { name: String },
    Variable,
}
