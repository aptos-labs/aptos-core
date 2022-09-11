// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::aptos_api_types::U64;
use bigdecimal::{FromPrimitive, Signed, ToPrimitive, Zero};

pub fn u64_to_bigdecimal(val: u64) -> bigdecimal::BigDecimal {
    bigdecimal::BigDecimal::from_u64(val).expect("Unable to convert u64 to big decimal")
}

pub fn bigdecimal_to_u64(val: &bigdecimal::BigDecimal) -> u64 {
    val.to_u64().expect("Unable to convert big decimal to u64")
}

pub fn ensure_not_negative(val: bigdecimal::BigDecimal) -> bigdecimal::BigDecimal {
    if val.is_negative() {
        return bigdecimal::BigDecimal::zero();
    }
    val
}

pub fn parse_timestamp(ts: U64, version: i64) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt((*ts.inner() / 1000000) as i64, 0)
        .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}

pub fn parse_timestamp_secs(ts: U64, version: i64) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt(
        std::cmp::min(*ts.inner(), chrono::NaiveDateTime::MAX.timestamp() as u64) as i64,
        0,
    )
    .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_timestamp() {
        let current_year = chrono::offset::Utc::now().year();

        let ts = parse_timestamp(U64::from(1649560602763949), 1);
        assert_eq!(ts.timestamp(), 1649560602);
        assert_eq!(ts.year(), current_year);

        let ts2 = parse_timestamp_secs(U64::from(600000000000000), 2);
        assert_eq!(ts2.year(), current_year + 10);

        let ts3 = parse_timestamp_secs(U64::from(1659386386), 2);
        assert_eq!(ts3.timestamp(), 1659386386);
    }
}
