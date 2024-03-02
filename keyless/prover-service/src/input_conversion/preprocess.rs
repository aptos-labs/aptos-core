use super::types::Input;
use crate::api::{AsFr as _, RequestInput};
use anyhow::bail;
use std::collections::HashMap;

pub fn decode_and_add_jwk(rqi: RequestInput) -> Result<Input, anyhow::Error> {
    if rqi.aud_override.is_some() {
        bail!("aud_override is unsupported for now")
    } else {
        let extra_field_jwt_key = match &rqi.extra_field {
            Some(x) => String::from(x),
            None => String::from(""),
        };

        Ok(Input {
            jwt_b64: rqi.jwt_b64,
            epk: rqi.epk,
            epk_blinder_fr: rqi.epk_blinder.as_fr(),
            exp_date_secs: rqi.exp_date_secs,
            pepper_fr: rqi.pepper.as_fr(),
            variable_keys: HashMap::from([
                (String::from("uid"), rqi.uid_key),
                (String::from("extra"), extra_field_jwt_key),
            ]),
            exp_horizon_secs: rqi.exp_horizon_secs,
            use_extra_field: rqi.extra_field.is_some(),
        })
    }
}
