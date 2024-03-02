use super::{
    circuit_input_signals::{CircuitInputSignals, Padded},
    field_parser::FieldCheckInput,
};
use crate::input_conversion::{
    config::{CircuitConfig, FieldCheckInputConfig, Key},
    field_parser::FieldParser,
    types::Ascii,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

//#[derive(Debug)]
//pub struct FieldCheckInput {
//    pub index : usize,
//    pub key : String,
//    pub value : String,
//    pub colon_index : usize,
//    pub value_index : usize,
//    pub whole_field : String
//}
//
//
//pub fn parse_field(jwt_payload: &Ascii, key: &str) -> Option<FieldCheckInput> {
//    let key_in_quotes = String::from("\"") + &key + "\"";
//    println!("key: {}", key);
//    let index = jwt_payload.find(&key_in_quotes)?;
//    let colon_index = jwt_payload.find_starting_at(index, ":")?;
//    println!("colon index: {}", colon_index);
//    let mut value_index = jwt_payload.first_non_space_char_starting_at(colon_index+1)?;
//    println!("value index: {}", value_index);
//    let (value, value_end) = jwt_payload.value_starting_at(value_index)?;
//    if jwt_payload.as_bytes()[value_index] == ('"' as u8) {
//        value_index += 1;
//    }
//    println!("value: {}", value);
//    let whole_field = jwt_payload.whole_field(index, value_end)?;
//
//    Some(
//        FieldCheckInput {
//            index,
//            key : String::from(key),
//            value,
//            colon_index: colon_index - index, // index is local not global
//            value_index: value_index - index, // again, local not global
//            whole_field
//        }
//        )
//}
//

pub fn padded_field_check_input_signals(
    jwt_payload: &str,
    _global_config: &CircuitConfig,
    config: &FieldCheckInputConfig,
    variable_keys: &HashMap<String, String>,
) -> Result<CircuitInputSignals<Padded>> {
    let name = &config.circuit_input_signal_prefix;
    let key_in_payload = match &config.jwt_key {
        Key::Fixed { name } => name.clone(),
        Key::Variable => variable_keys
            .get(&config.circuit_input_signal_prefix)
            .ok_or(anyhow!(
                "Did not find key {} in variable_keys",
                &config.circuit_input_signal_prefix
            ))?
            .clone(),
    };

    let parsed_input;
    if name == "ev" {
        println!(
            "uid key: {}",
            variable_keys.get("uid").ok_or(anyhow!(
                "Did not find key {} in variable_keys",
                &config.circuit_input_signal_prefix
            ))?
        );
        if variable_keys.get("uid").ok_or(anyhow!(
            "Did not find key {} in variable_keys",
            &config.circuit_input_signal_prefix
        ))? != "email"
        {
            println!("disabling ev field");
            parsed_input = FieldCheckInput {
                index: 0,
                key: String::from("email_verified"),
                value: String::from("true"),
                colon_index: 16,
                value_index: 17,
                whole_field: String::from("\"email_verified\":true,"),
            }
        } else {
            parsed_input = FieldParser::find_and_parse_field(jwt_payload, &key_in_payload)?;
        }
    } else {
        parsed_input = FieldParser::find_and_parse_field(jwt_payload, &key_in_payload)?;
    }

    let _name_string = String::from(name);
    let mut result: CircuitInputSignals<Padded> = CircuitInputSignals::new_padded()
        .bytes_input_padded(
            &(String::from(name) + "_field"),
            Ascii::from(parsed_input.whole_field.as_str())
                .pad(config.max_whole_field_length)?
                .as_bytes(),
        )
        .usize_input_padded(
            &(String::from(name) + "_field_len"),
            parsed_input.whole_field.len(),
        )
        .usize_input_padded(&(String::from(name) + "_index"), parsed_input.index);

    if config.has_value_inputs {
        if let Key::Variable = config.jwt_key {
            result = result
                .usize_input_padded(&(String::from(name) + "_name_len"), key_in_payload.len());
        }

        result = result
            .usize_input_padded(
                &(String::from(name) + "_colon_index"),
                parsed_input.colon_index,
            )
            .bytes_input_padded(
                &(String::from(name) + "_name"),
                &Vec::from(
                    Ascii::from(key_in_payload.as_str())
                        .pad(config.max_name_length)?
                        .as_bytes(),
                ),
            )
            .usize_input_padded(
                &(String::from(name) + "_value_index"),
                parsed_input.value_index,
            );

        if key_in_payload != "aud" {
            result = result
                .usize_input_padded(
                    &(String::from(name) + "_value_len"),
                    parsed_input.value.len(),
                )
                .bytes_input_padded(
                    &(String::from(name) + "_value"),
                    &Vec::from(
                        Ascii::from(parsed_input.value.as_str())
                            .pad(config.max_value_length)?
                            .as_bytes(),
                    ),
                );
        }
    }

    // special logic for aud
    if key_in_payload == "aud" {
        result = result
            .usize_input_padded("private_aud_value_len", parsed_input.value.len())
            .bytes_input_padded(
                "private_aud_value",
                &Vec::from(
                    Ascii::from(parsed_input.value.as_str())
                        .pad(config.max_value_length)?
                        .as_bytes(),
                ),
            )
            .usize_input_padded("override_aud_value_len", 0)
            .bytes_input_padded(
                "override_aud_value",
                &Vec::from(Ascii::from("").pad(config.max_value_length)?.as_bytes()),
            )
            .bool_input_padded("use_aud_override", false);
    }

    Ok(result)
}
