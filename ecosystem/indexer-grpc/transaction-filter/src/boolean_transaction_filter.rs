// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    errors::FilterError,
    filters::{EventFilter, TransactionRootFilter, UserTransactionFilter},
    traits::Filterable,
};
use anyhow::{anyhow, ensure, Result};
use aptos_protos::transaction::v1::{transaction::TxnData, Transaction};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// BooleanTransactionFilter is the top level filter
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BooleanTransactionFilter {
    And(LogicalAnd),
    Or(LogicalOr),
    Not(LogicalNot),
    Filter(APIFilter),
}

impl From<APIFilter> for BooleanTransactionFilter {
    fn from(filter: APIFilter) -> Self {
        BooleanTransactionFilter::Filter(filter)
    }
}

impl From<TransactionRootFilter> for BooleanTransactionFilter {
    fn from(filter: TransactionRootFilter) -> Self {
        BooleanTransactionFilter::Filter(APIFilter::TransactionRootFilter(filter))
    }
}

impl From<UserTransactionFilter> for BooleanTransactionFilter {
    fn from(filter: UserTransactionFilter) -> Self {
        BooleanTransactionFilter::Filter(APIFilter::UserTransactionFilter(filter))
    }
}

impl From<EventFilter> for BooleanTransactionFilter {
    fn from(filter: EventFilter) -> Self {
        BooleanTransactionFilter::Filter(APIFilter::EventFilter(filter))
    }
}

impl From<BooleanTransactionFilter> for aptos_protos::indexer::v1::BooleanTransactionFilter {
    fn from(boolean_transaction_filter: BooleanTransactionFilter) -> Self {
        match boolean_transaction_filter {
            BooleanTransactionFilter::And(logical_and) => {
                aptos_protos::indexer::v1::BooleanTransactionFilter {
                    filter: Some(
                        aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalAnd(
                            logical_and.into(),
                        ),
                    ),
                }
            },
            BooleanTransactionFilter::Or(logical_or) => {
                aptos_protos::indexer::v1::BooleanTransactionFilter {
                    filter: Some(
                        aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalOr(
                            logical_or.into(),
                        ),
                    ),
                }
            },
            BooleanTransactionFilter::Not(logical_not) => {
                aptos_protos::indexer::v1::BooleanTransactionFilter {
                    filter: Some(
                        aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalNot(
                            // We do not `impl From` for `LogicalNot` because it is a recursive type that only contains a `Box<BooleanTransactionFilter>`
                            Box::new((*logical_not.not).into()),
                        ),
                    ),
                }
            },
            BooleanTransactionFilter::Filter(api_filter) => {
                aptos_protos::indexer::v1::BooleanTransactionFilter {
                    filter: Some(
                        aptos_protos::indexer::v1::boolean_transaction_filter::Filter::ApiFilter(
                            api_filter.into(),
                        ),
                    ),
                }
            },
        }
    }
}

impl BooleanTransactionFilter {
    pub fn new_from_proto(
        proto_filter: aptos_protos::indexer::v1::BooleanTransactionFilter,
        max_filter_size: Option<usize>,
    ) -> Result<Self> {
        if let Some(max_filter_size) = max_filter_size {
            ensure!(
                proto_filter.encoded_len() <= max_filter_size,
                format!(
                    "Filter is too complicated. Max size: {} bytes, Actual size: {} bytes",
                    max_filter_size,
                    proto_filter.encoded_len()
                )
            );
        }
        Ok(
            match proto_filter
                .filter
                .ok_or(anyhow!("Oneof is not set in BooleanTransactionFilter."))?
            {
                aptos_protos::indexer::v1::boolean_transaction_filter::Filter::ApiFilter(
                    api_filter,
                ) => TryInto::<APIFilter>::try_into(api_filter)?.into(),
                aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalAnd(
                    logical_and,
                ) => BooleanTransactionFilter::And(logical_and.try_into()?),
                aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalOr(
                    logical_or,
                ) => BooleanTransactionFilter::Or(logical_or.try_into()?),
                aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalNot(
                    logical_not,
                ) => BooleanTransactionFilter::Not(logical_not.try_into()?),
            },
        )
    }

    pub fn into_proto(self) -> aptos_protos::indexer::v1::BooleanTransactionFilter {
        self.into()
    }

    /// Combines the current filter with another filter using a logical AND.
    /// Returns a new `BooleanTransactionFilter` representing the conjunction.
    ///
    /// # Example
    ///
    /// ```
    /// use aptos_transaction_filter::BooleanTransactionFilter;
    /// use aptos_transaction_filter::filters::{EventFilterBuilder, MoveStructTagFilterBuilder, UserTransactionFilterBuilder};
    ///
    /// fn example() -> Result<BooleanTransactionFilter, anyhow::Error> {
    ///   // Create a filter for user transactions where the sender is "0x1"
    ///   let user_sender_filter = UserTransactionFilterBuilder::default()
    ///       .sender("0x1")
    ///       .build()?;
    ///   // Create a filter where the event struct address Is 0x0fff
    ///   let event_filter = EventFilterBuilder::default()
    ///       .struct_type(
    ///           MoveStructTagFilterBuilder::default()
    ///           .address("0x0fff").build()?
    ///       ).build()?;
    ///   // Combine the two using logical AND
    ///  Ok(BooleanTransactionFilter::from(user_sender_filter).and(event_filter))
    /// }
    /// ```
    pub fn and<Other: Into<BooleanTransactionFilter>>(self, other: Other) -> Self {
        BooleanTransactionFilter::And(LogicalAnd {
            and: vec![self, other.into()],
        })
    }

    /// Combines the current filter with another filter using a logical OR.
    /// Returns a new `BooleanTransactionFilter` representing the disjunction.
    ///
    /// # Example
    ///
    /// ```
    /// use aptos_transaction_filter::BooleanTransactionFilter;
    /// use aptos_transaction_filter::filters::{EventFilterBuilder, MoveStructTagFilterBuilder, UserTransactionFilterBuilder};
    ///
    /// fn example() -> Result<BooleanTransactionFilter, anyhow::Error> {
    ///   // Create a filter for user transactions where the sender is "0x1"
    ///   let user_sender_filter = UserTransactionFilterBuilder::default()
    ///       .sender("0x1")
    ///       .build()?;
    ///   // Create a filter where the event struct address Is 0x0fff
    ///   let event_filter = EventFilterBuilder::default()
    ///       .struct_type(
    ///           MoveStructTagFilterBuilder::default()
    ///           .address("0x0fff").build()?
    ///       ).build()?;
    ///   // Combine the two using logical OR
    ///   Ok(BooleanTransactionFilter::from(user_sender_filter).or(event_filter))
    /// }
    /// ```
    pub fn or<Other: Into<BooleanTransactionFilter>>(self, other: Other) -> Self {
        BooleanTransactionFilter::Or(LogicalOr {
            or: vec![self, other.into()],
        })
    }

    /// Negates the current filter.
    /// Returns a new `BooleanTransactionFilter` representing the negation.
    ///
    /// # Example
    ///
    /// ```
    /// use aptos_transaction_filter::BooleanTransactionFilter;
    /// use aptos_transaction_filter::filters::{EventFilterBuilder, MoveStructTagFilterBuilder, UserTransactionFilterBuilder};
    ///
    /// fn example() -> Result<BooleanTransactionFilter, anyhow::Error> {
    ///   // Create a filter for user transactions where the sender is "0x1"
    ///   let user_sender_filter = UserTransactionFilterBuilder::default()
    ///       .sender("0x1")
    ///       .build()?;
    ///
    ///   // Negate the filter; this now matches transactions where the submitter != 0x1
    ///   Ok(BooleanTransactionFilter::from(user_sender_filter).not())
    /// }
    /// ```
    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Self {
        BooleanTransactionFilter::Not(LogicalNot {
            not: Box::new(self),
        })
    }

    /// Creates a new `BooleanTransactionFilter` representing a conjunction of multiple filters.
    pub fn new_and(and: Vec<BooleanTransactionFilter>) -> Self {
        BooleanTransactionFilter::And(LogicalAnd { and })
    }

    /// Creates a new `BooleanTransactionFilter` representing a disjunction of multiple filters.
    pub fn new_or(or: Vec<BooleanTransactionFilter>) -> Self {
        BooleanTransactionFilter::Or(LogicalOr { or })
    }

    /// Creates a new `BooleanTransactionFilter` representing the negation of a filter.
    pub fn new_not(not: BooleanTransactionFilter) -> Self {
        BooleanTransactionFilter::Not(LogicalNot { not: Box::new(not) })
    }

    /// Wraps an APIFilter into a `BooleanTransactionFilter`.
    pub fn new_filter(filter: APIFilter) -> Self {
        BooleanTransactionFilter::Filter(filter)
    }
}

impl Filterable<Transaction> for BooleanTransactionFilter {
    fn validate_state(&self) -> Result<(), FilterError> {
        match self {
            BooleanTransactionFilter::And(and) => and.is_valid(),
            BooleanTransactionFilter::Or(or) => or.is_valid(),
            BooleanTransactionFilter::Not(not) => not.is_valid(),
            BooleanTransactionFilter::Filter(filter) => filter.is_valid(),
        }
    }

    fn matches(&self, item: &Transaction) -> bool {
        match self {
            BooleanTransactionFilter::And(and) => and.matches(item),
            BooleanTransactionFilter::Or(or) => or.matches(item),
            BooleanTransactionFilter::Not(not) => not.matches(item),
            BooleanTransactionFilter::Filter(filter) => filter.matches(item),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogicalAnd {
    and: Vec<BooleanTransactionFilter>,
}

impl TryFrom<aptos_protos::indexer::v1::LogicalAndFilters> for LogicalAnd {
    type Error = anyhow::Error;

    fn try_from(proto_filter: aptos_protos::indexer::v1::LogicalAndFilters) -> Result<Self> {
        Ok(Self {
            and: proto_filter
                .filters
                .into_iter()
                .map(|f| BooleanTransactionFilter::new_from_proto(f, None))
                .collect::<Result<_>>()?,
        })
    }
}

impl From<LogicalAnd> for aptos_protos::indexer::v1::LogicalAndFilters {
    fn from(logical_and: LogicalAnd) -> Self {
        aptos_protos::indexer::v1::LogicalAndFilters {
            filters: logical_and.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl Filterable<Transaction> for LogicalAnd {
    fn validate_state(&self) -> Result<(), FilterError> {
        for filter in &self.and {
            filter.is_valid()?;
        }
        Ok(())
    }

    fn matches(&self, item: &Transaction) -> bool {
        self.and.iter().all(|filter| filter.matches(item))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogicalOr {
    or: Vec<BooleanTransactionFilter>,
}

impl TryFrom<aptos_protos::indexer::v1::LogicalOrFilters> for LogicalOr {
    type Error = anyhow::Error;

    fn try_from(proto_filter: aptos_protos::indexer::v1::LogicalOrFilters) -> Result<Self> {
        Ok(Self {
            or: proto_filter
                .filters
                .into_iter()
                .map(|f| BooleanTransactionFilter::new_from_proto(f, None))
                .collect::<Result<_>>()?,
        })
    }
}

impl From<LogicalOr> for aptos_protos::indexer::v1::LogicalOrFilters {
    fn from(logical_or: LogicalOr) -> Self {
        aptos_protos::indexer::v1::LogicalOrFilters {
            filters: logical_or.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl Filterable<Transaction> for LogicalOr {
    fn validate_state(&self) -> Result<(), FilterError> {
        for filter in &self.or {
            filter.is_valid()?;
        }
        Ok(())
    }

    fn matches(&self, item: &Transaction) -> bool {
        self.or.iter().any(|filter| filter.matches(item))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogicalNot {
    not: Box<BooleanTransactionFilter>,
}

impl TryFrom<Box<aptos_protos::indexer::v1::BooleanTransactionFilter>> for LogicalNot {
    type Error = anyhow::Error;

    fn try_from(
        proto_filter: Box<aptos_protos::indexer::v1::BooleanTransactionFilter>,
    ) -> Result<Self> {
        Ok(Self {
            not: Box::new(BooleanTransactionFilter::new_from_proto(
                *proto_filter,
                None,
            )?),
        })
    }
}

impl Filterable<Transaction> for LogicalNot {
    fn validate_state(&self) -> Result<(), FilterError> {
        self.not.is_valid()
    }

    fn matches(&self, item: &Transaction) -> bool {
        !self.not.matches(item)
    }
}

/// These are filters we would expect to be exposed via API
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum APIFilter {
    TransactionRootFilter(TransactionRootFilter),
    UserTransactionFilter(UserTransactionFilter),
    EventFilter(EventFilter),
}

impl TryFrom<aptos_protos::indexer::v1::ApiFilter> for APIFilter {
    type Error = anyhow::Error;

    fn try_from(proto_filter: aptos_protos::indexer::v1::ApiFilter) -> Result<Self> {
        Ok(
            match proto_filter
                .filter
                .ok_or(anyhow!("Oneof is not set in ApiFilter."))?
            {
                aptos_protos::indexer::v1::api_filter::Filter::TransactionRootFilter(
                    transaction_root_filter,
                ) => Into::<TransactionRootFilter>::into(transaction_root_filter).into(),
                aptos_protos::indexer::v1::api_filter::Filter::UserTransactionFilter(
                    user_transaction_filter,
                ) => Into::<UserTransactionFilter>::into(user_transaction_filter).into(),
                aptos_protos::indexer::v1::api_filter::Filter::EventFilter(event_filter) => {
                    Into::<EventFilter>::into(event_filter).into()
                },
            },
        )
    }
}

impl From<TransactionRootFilter> for APIFilter {
    fn from(filter: TransactionRootFilter) -> Self {
        APIFilter::TransactionRootFilter(filter)
    }
}

impl From<UserTransactionFilter> for APIFilter {
    fn from(filter: UserTransactionFilter) -> Self {
        APIFilter::UserTransactionFilter(filter)
    }
}

impl From<EventFilter> for APIFilter {
    fn from(filter: EventFilter) -> Self {
        APIFilter::EventFilter(filter)
    }
}

impl From<APIFilter> for aptos_protos::indexer::v1::ApiFilter {
    fn from(api_filter: APIFilter) -> Self {
        match api_filter {
            APIFilter::TransactionRootFilter(transaction_root_filter) => {
                aptos_protos::indexer::v1::ApiFilter {
                    filter: Some(
                        aptos_protos::indexer::v1::api_filter::Filter::TransactionRootFilter(
                            transaction_root_filter.into(),
                        ),
                    ),
                }
            },
            APIFilter::UserTransactionFilter(user_transaction_filter) => {
                aptos_protos::indexer::v1::ApiFilter {
                    filter: Some(
                        aptos_protos::indexer::v1::api_filter::Filter::UserTransactionFilter(
                            user_transaction_filter.into(),
                        ),
                    ),
                }
            },
            APIFilter::EventFilter(event_filter) => aptos_protos::indexer::v1::ApiFilter {
                filter: Some(aptos_protos::indexer::v1::api_filter::Filter::EventFilter(
                    aptos_protos::indexer::v1::EventFilter {
                        struct_type: event_filter.struct_type.map(Into::into),
                        data_substring_filter: event_filter.data_substring_filter,
                    },
                )),
            },
        }
    }
}

impl Filterable<Transaction> for APIFilter {
    fn validate_state(&self) -> Result<(), FilterError> {
        match self {
            APIFilter::TransactionRootFilter(filter) => filter.is_valid(),
            APIFilter::UserTransactionFilter(filter) => filter.is_valid(),
            APIFilter::EventFilter(filter) => filter.is_valid(),
        }
    }

    fn matches(&self, txn: &Transaction) -> bool {
        match self {
            APIFilter::TransactionRootFilter(filter) => filter.matches(txn),
            APIFilter::UserTransactionFilter(ut_filter) => ut_filter.matches(txn),
            APIFilter::EventFilter(events_filter) => {
                if let Some(txn_data) = &txn.txn_data {
                    let events = match txn_data {
                        TxnData::BlockMetadata(bm) => &bm.events,
                        TxnData::Genesis(g) => &g.events,
                        TxnData::StateCheckpoint(_) => return false,
                        TxnData::User(u) => &u.events,
                        TxnData::Validator(_) => return false,
                        TxnData::BlockEpilogue(_) => return false,
                        TxnData::ScheduledTransaction(s) => &s.events,
                    };
                    events_filter.matches_vec(events)
                } else {
                    false
                }
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::filters::{
        event::EventFilterBuilder, move_module::MoveStructTagFilterBuilder,
        /*user_transaction::EntryFunctionFilter,*/ TransactionRootFilterBuilder,
        UserTransactionFilterBuilder, /*UserTransactionPayloadFilterBuilder,*/
    };

    // Disabled for now while we investigate an issue with lz4 in aptos-core:
    // https://aptos-org.slack.com/archives/C04PF1X2UKY/p1718995777239809?thread_ts=1718969817.705389&cid=C04PF1X2UKY
    /*
    #[test]
    pub fn test_query_parsing() {
        let trf = TransactionRootFilter {
            success: Some(true),
            txn_type: Some(aptos_protos::transaction::v1::transaction::TransactionType::User),
        };

        let utf = UserTransactionFilterBuilder::default()
            .sender("0x0011")
            .payload(
                UserTransactionPayloadFilterBuilder::default()
                    .function(EntryFunctionFilter {
                        address: Some("0x007".into()),
                        module: Some("roulette".into()),
                        function: None,
                    })
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let ef = EventFilterBuilder::default()
            .struct_type(
                MoveStructTagFilterBuilder::default()
                    .address("0x0077")
                    .module("roulette")
                    .name("spin")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // (trf OR utf)
        let trf_or_utf = BooleanTransactionFilter::from(trf).or(utf);
        // ((trf OR utf) AND ef)
        let query = trf_or_utf.and(ef);

        println!(
            "JSON RESULT (QUERY 1):\n {}",
            serde_json::to_string_pretty(&query).unwrap()
        );

        let txns = load_graffio_fixture();

        // Benchmark how long it takes to do this 100 times
        let start = std::time::Instant::now();
        const LOOPS: i32 = 1000;
        for _ in 0..LOOPS {
            for txn in &txns.transactions {
                query.matches(txn);
            }
        }
        let elapsed = start.elapsed();

        let total_txn = LOOPS * txns.transactions.len() as i32;
        println!(
            "BENCH: Took {:?} for {} transactions ({:?} each)",
            elapsed,
            total_txn,
            elapsed / total_txn as u32
        );

        let ef_econia = EventFilterBuilder::default()
            .struct_type(
                MoveStructTagFilterBuilder::default()
                    .address("0x00ECONIA")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let ef_aries = EventFilterBuilder::default()
            .struct_type(
                MoveStructTagFilterBuilder::default()
                    .address("0x00ARIES")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let query = BooleanTransactionFilter::from(ef_econia).or(ef_aries);
        println!(
            "JSON RESULT (QUERY 2):\n {}",
            serde_json::to_string_pretty(&query).unwrap()
        );
    }
    */

    #[test]
    fn test_serialization() {
        let trf = TransactionRootFilterBuilder::default()
            .success(true)
            .build()
            .unwrap();

        let utf = UserTransactionFilterBuilder::default()
            .sender("0x0011")
            .build()
            .unwrap();

        let ef = EventFilterBuilder::default()
            .struct_type(
                MoveStructTagFilterBuilder::default()
                    .address("0x0077")
                    .module("roulette")
                    .name("spin")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        // Combine filters using logical operators!
        // (trf OR utf)
        let trf_or_utf = BooleanTransactionFilter::from(trf).or(utf);
        // ((trf OR utf) AND ef)
        let query = trf_or_utf.and(ef);

        let yaml = serde_yaml::to_string(&query).unwrap();
        println!("YAML: \n{}", yaml);
    }

    #[test]
    fn test_into_proto() {
        let filter = BooleanTransactionFilter::new_and(vec![
            BooleanTransactionFilter::new_or(vec![
                BooleanTransactionFilter::new_filter(APIFilter::TransactionRootFilter(
                    TransactionRootFilterBuilder::default()
                        .success(true)
                        .build()
                        .unwrap(),
                )),
                BooleanTransactionFilter::new_filter(APIFilter::UserTransactionFilter(
                    UserTransactionFilterBuilder::default()
                        .sender("0x0011")
                        .build()
                        .unwrap(),
                )),
            ]),
            BooleanTransactionFilter::new_filter(APIFilter::EventFilter(
                EventFilterBuilder::default()
                    .struct_type(
                        MoveStructTagFilterBuilder::default()
                            .address("0x0077")
                            .module("roulette")
                            .name("spin")
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            )),
            BooleanTransactionFilter::new_not(BooleanTransactionFilter::new_filter(
                APIFilter::EventFilter(
                    EventFilterBuilder::default()
                        .struct_type(
                            MoveStructTagFilterBuilder::default()
                                .address("0x0088")
                                .build()
                                .unwrap(),
                        )
                        .build()
                        .unwrap(),
                ),
            )),
        ]);

        let expected_proto = aptos_protos::indexer::v1::BooleanTransactionFilter {
            filter: Some(
                aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalAnd(
                    aptos_protos::indexer::v1::LogicalAndFilters {
                        filters: vec![
                            aptos_protos::indexer::v1::BooleanTransactionFilter {
                                filter: Some(
                                    aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalOr(
                                        aptos_protos::indexer::v1::LogicalOrFilters {
                                            filters: vec![
                                                aptos_protos::indexer::v1::BooleanTransactionFilter {
                                                    filter: Some(
                                                        aptos_protos::indexer::v1::boolean_transaction_filter::Filter::ApiFilter(
                                                            aptos_protos::indexer::v1::ApiFilter {
                                                                filter: Some(
                                                                    aptos_protos::indexer::v1::api_filter::Filter::TransactionRootFilter(
                                                                        aptos_protos::indexer::v1::TransactionRootFilter {
                                                                            success: Some(true),
                                                                            transaction_type: None,
                                                                        },
                                                                    ),
                                                                ),
                                                            },
                                                        ),
                                                    ),
                                                },
                                                aptos_protos::indexer::v1::BooleanTransactionFilter {
                                                    filter: Some(
                                                        aptos_protos::indexer::v1::boolean_transaction_filter::Filter::ApiFilter(
                                                            aptos_protos::indexer::v1::ApiFilter {
                                                                filter: Some(
                                                                    aptos_protos::indexer::v1::api_filter::Filter::UserTransactionFilter(
                                                                        aptos_protos::indexer::v1::UserTransactionFilter {
                                                                            sender: Some(
                                                                                "0x0011".to_string()
                                                                            ),
                                                                            payload_filter: None,
                                                                        },
                                                                    ),
                                                                ),
                                                            },
                                                        ),
                                                    ),
                                                },
                                            ],
                                        },
                                    ),
                                ),
                            },
                            aptos_protos::indexer::v1::BooleanTransactionFilter {
                                filter: Some(
                                    aptos_protos::indexer::v1::boolean_transaction_filter::Filter::ApiFilter(
                                        aptos_protos::indexer::v1::ApiFilter {
                                            filter: Some(
                                                aptos_protos::indexer::v1::api_filter::Filter::EventFilter(
                                                    aptos_protos::indexer::v1::EventFilter {
                                                        struct_type: Some(
                                                            aptos_protos::indexer::v1::MoveStructTagFilter {
                                                                address: Some("0x0077".to_string()),
                                                                module: Some("roulette".to_string()),
                                                                name: Some("spin".to_string()),
                                                            },
                                                        ),
                                                        data_substring_filter: None,
                                                    },
                                                ),
                                            ),
                                        },
                                    ),
                                ),
                            },
                            aptos_protos::indexer::v1::BooleanTransactionFilter {
                                filter: Some(
                                    aptos_protos::indexer::v1::boolean_transaction_filter::Filter::LogicalNot(
                                        Box::new(
                                            aptos_protos::indexer::v1::BooleanTransactionFilter {
                                                filter: Some(
                                                    aptos_protos::indexer::v1::boolean_transaction_filter::Filter::ApiFilter(
                                                        aptos_protos::indexer::v1::ApiFilter {
                                                            filter: Some(
                                                                aptos_protos::indexer::v1::api_filter::Filter::EventFilter(
                                                                    aptos_protos::indexer::v1::EventFilter {
                                                                        struct_type: Some(
                                                                            aptos_protos::indexer::v1::MoveStructTagFilter {
                                                                                address: Some("0x0088".to_string()),
                                                                                module: None,
                                                                                name: None,
                                                                            },
                                                                        ),
                                                                        data_substring_filter: None,
                                                                    },
                                                                ),
                                                            ),
                                                        },
                                                    ),
                                                ),
                                            },
                                        ),
                                    ),
                                ),
                            },
                        ],
                    },
                ),
            ),
        };

        let actual_proto = filter.into_proto();
        assert_eq!(expected_proto, actual_proto);
    }
}
