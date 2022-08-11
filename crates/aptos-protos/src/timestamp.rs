// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::pb::aptos::util::timestamp::Timestamp;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::convert::{TryFrom, TryInto};

impl TryFrom<Timestamp> for chrono::DateTime<Utc> {
    type Error = std::num::TryFromIntError;
    fn try_from(value: Timestamp) -> Result<Self, Self::Error> {
        let Timestamp { seconds, nanos } = value;

        let dt = NaiveDateTime::from_timestamp(seconds, nanos.try_into()?);
        Ok(Self::from_utc(dt, Utc))
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self {
            seconds: value.timestamp(),
            nanos: value.timestamp_subsec_nanos() as i32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    #[test]
    fn test_date() {
        let datetime = FixedOffset::east(5 * 3600)
            .ymd(2016, 11, 8)
            .and_hms(21, 7, 9);
        let encoded = datetime.to_rfc3339();
        assert_eq!(&encoded, "2016-11-08T21:07:09+05:00");

        let utc: DateTime<Utc> = datetime.into();
        let utc_encoded = utc.to_rfc3339();
        assert_eq!(&utc_encoded, "2016-11-08T16:07:09+00:00");

        let a: Timestamp = Timestamp::from(utc);
        assert_eq!(a.seconds, utc.timestamp());
        assert_eq!(a.nanos, utc.timestamp_subsec_nanos() as i32);

        let t: DateTime<Utc> = DateTime::<Utc>::try_from(a).unwrap();
        assert_eq!(t.timestamp(), utc.timestamp());
        assert_eq!(t.timestamp_subsec_nanos(), utc.timestamp_subsec_nanos());
    }
}
