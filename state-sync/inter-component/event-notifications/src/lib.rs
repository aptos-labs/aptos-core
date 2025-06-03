// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
use anyhow::{anyhow, Result};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_id_generator::{IdGenerator, U64IdGenerator};
use aptos_infallible::RwLock;
use aptos_storage_interface::{
    state_store::state_view::db_state_view::DbStateViewAtVersion, DbReader, DbReaderWriter,
};
use aptos_types::{
    contract_event::ContractEvent,
    event::EventKey,
    on_chain_config::{
        ConfigurationResource, OnChainConfig, OnChainConfigPayload, OnChainConfigProvider,
    },
    state_store::state_key::StateKey,
    transaction::Version,
};
use futures::{channel::mpsc::SendError, stream::FusedStream, Stream};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    iter::FromIterator,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use thiserror::Error;

#[cfg(test)]
mod tests;

// Maximum channel sizes for each notification subscriber. If messages are not
// consumed, they will be dropped (oldest messages first). The remaining messages
// will be retrieved using FIFO ordering.
const EVENT_NOTIFICATION_CHANNEL_SIZE: usize = 100;
const RECONFIG_NOTIFICATION_CHANNEL_SIZE: usize = 1; // Note: this should be 1 to ensure only the latest reconfig is consumed

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Cannot subscribe to zero event keys!")]
    CannotSubscribeToZeroEventKeys,
    #[error("Missing event subscription! Subscription ID: {0}")]
    MissingEventSubscription(u64),
    #[error("Unable to send event notification! Error: {0}")]
    UnableToSendEventNotification(String),
    #[error("Unexpected error encountered: {0}")]
    UnexpectedErrorEncountered(String),
}

impl From<SendError> for Error {
    fn from(error: SendError) -> Self {
        Error::UnableToSendEventNotification(error.to_string())
    }
}

/// The interface between state sync and the subscription notification service,
/// allowing state sync to notify the subscription service of new events.
pub trait EventNotificationSender: Send {
    /// Notify the subscription service of the events at the specified version.
    fn notify_events(&mut self, version: Version, events: Vec<ContractEvent>) -> Result<(), Error>;

    /// Forces the subscription service to notify subscribers of the current
    /// on-chain configurations at the specified version.
    /// This is useful for forcing reconfiguration notifications even if no
    /// reconfiguration event was processed (e.g., on startup).
    fn notify_initial_configs(&mut self, version: Version) -> Result<(), Error>;
}

/// The subscription service offered by state sync, responsible for notifying
/// subscribers of on-chain events.
pub struct EventSubscriptionService {
    // Event subscription registry
    event_key_subscriptions: HashMap<EventKey, HashSet<SubscriptionId>>,
    event_v2_tag_subscriptions: HashMap<String, HashSet<SubscriptionId>>,
    subscription_id_to_event_subscription: HashMap<SubscriptionId, EventSubscription>,

    // Reconfig subscription registry
    reconfig_subscriptions: HashMap<SubscriptionId, ReconfigSubscription>,

    // Database to fetch on-chain configuration data
    storage: Arc<RwLock<DbReaderWriter>>,

    // Internal subscription ID generator
    subscription_id_generator: U64IdGenerator,
}

impl EventSubscriptionService {
    pub fn new(storage: Arc<RwLock<DbReaderWriter>>) -> Self {
        Self {
            event_key_subscriptions: HashMap::new(),
            event_v2_tag_subscriptions: HashMap::new(),
            subscription_id_to_event_subscription: HashMap::new(),
            reconfig_subscriptions: HashMap::new(),
            storage,
            subscription_id_generator: U64IdGenerator::new(),
        }
    }

    /// Returns an EventNotificationListener that can be monitored for
    /// subscribed events. If an event key is subscribed to, it means the
    /// EventNotificationListener will be sent a notification every time an
    /// event with the matching key occurs on-chain. Note: if the notification
    /// buffer fills up too quickly, older notifications will be dropped. As
    /// such, it is the responsibility of the subscriber to ensure notifications
    /// are processed in a timely manner.
    pub fn subscribe_to_events(
        &mut self,
        event_keys: Vec<EventKey>,
        event_v2_tags: Vec<String>,
    ) -> Result<EventNotificationListener, Error> {
        if event_keys.is_empty() && event_v2_tags.is_empty() {
            return Err(Error::CannotSubscribeToZeroEventKeys);
        }

        let (notification_sender, notification_receiver) =
            aptos_channel::new(QueueStyle::KLAST, EVENT_NOTIFICATION_CHANNEL_SIZE, None);

        // Create a new event subscription
        let subscription_id = self.get_new_subscription_id();
        let event_subscription = EventSubscription {
            notification_sender,
            event_buffer: vec![],
        };

        // Store the new subscription
        if let Some(old_subscription) = self
            .subscription_id_to_event_subscription
            .insert(subscription_id, event_subscription)
        {
            return Err(Error::UnexpectedErrorEncountered(format!(
                "Duplicate event subscription found! This should not occur! ID: {}, subscription: {:?}",
                subscription_id, old_subscription
            )));
        }

        // Update the event key subscriptions to include the new subscription
        for event_key in event_keys {
            self.event_key_subscriptions
                .entry(event_key)
                .and_modify(|subscriptions| {
                    subscriptions.insert(subscription_id);
                })
                .or_insert_with(|| HashSet::from_iter([subscription_id].iter().cloned()));
        }

        // Update the event v2 tag subscriptions to include the new subscription
        for event_tag in event_v2_tags {
            self.event_v2_tag_subscriptions
                .entry(event_tag)
                .and_modify(|subscriptions| {
                    subscriptions.insert(subscription_id);
                })
                .or_insert_with(|| HashSet::from_iter([subscription_id].iter().cloned()));
        }

        Ok(EventNotificationListener {
            notification_receiver,
        })
    }

    /// Returns a ReconfigNotificationListener that can be monitored for
    /// reconfiguration events. Subscribers will be sent a notification
    /// containing all new on-chain configuration values whenever a new epoch
    /// begins. Note: if the notification buffer fills up too quickly, older
    /// notifications will be dropped. As such, it is the responsibility of the
    /// subscriber to ensure notifications are processed in a timely manner.
    pub fn subscribe_to_reconfigurations(
        &mut self,
    ) -> Result<ReconfigNotificationListener<DbBackedOnChainConfig>, Error> {
        let (notification_sender, notification_receiver) =
            aptos_channel::new(QueueStyle::KLAST, RECONFIG_NOTIFICATION_CHANNEL_SIZE, None);

        // Create a new reconfiguration subscription
        let subscription_id = self.get_new_subscription_id();
        let reconfig_subscription = ReconfigSubscription {
            notification_sender,
        };

        // Store the new subscription
        if self
            .reconfig_subscriptions
            .insert(subscription_id, reconfig_subscription)
            .is_some()
        {
            return Err(Error::UnexpectedErrorEncountered(format!(
                "Duplicate reconfiguration subscription found! This should not occur! ID: {}",
                subscription_id,
            )));
        }

        Ok(ReconfigNotificationListener {
            notification_receiver,
        })
    }

    fn get_new_subscription_id(&mut self) -> u64 {
        self.subscription_id_generator.next()
    }

    /// This notifies all the event subscribers of the new events found at the
    /// specified version. If a reconfiguration event (i.e., new epoch) is found,
    /// this method will return true.
    fn notify_event_subscribers(
        &mut self,
        version: Version,
        events: Vec<ContractEvent>,
    ) -> Result<bool, Error> {
        let mut reconfig_event_found = false;
        let mut event_subscription_ids_to_notify = HashSet::new();

        for event in events.iter() {
            // Process all subscriptions for the current event
            let maybe_subscription_ids = match event {
                ContractEvent::V1(evt) => self.event_key_subscriptions.get(evt.key()),
                ContractEvent::V2(evt) => {
                    let tag = evt.type_tag().to_canonical_string();
                    self.event_v2_tag_subscriptions.get(&tag)
                },
            };
            if let Some(subscription_ids) = maybe_subscription_ids {
                // Add the event to the subscription's pending event buffer
                // and store the subscriptions that will need to notified once all
                // events have been processed.
                for subscription_id in subscription_ids.iter() {
                    if let Some(event_subscription) = self
                        .subscription_id_to_event_subscription
                        .get_mut(subscription_id)
                    {
                        event_subscription.buffer_event(event.clone());
                        event_subscription_ids_to_notify.insert(*subscription_id);
                    } else {
                        return Err(Error::MissingEventSubscription(*subscription_id));
                    }
                }
            }

            // Take note if a reconfiguration (new epoch) has occurred
            if event.is_new_epoch_event() {
                reconfig_event_found = true;
            }
        }

        // Notify event subscribers of the new events
        for event_subscription_id in event_subscription_ids_to_notify {
            if let Some(event_subscription) = self
                .subscription_id_to_event_subscription
                .get_mut(&event_subscription_id)
            {
                event_subscription.notify_subscriber_of_events(version)?;
            } else {
                return Err(Error::MissingEventSubscription(event_subscription_id));
            }
        }

        Ok(reconfig_event_found)
    }

    /// This notifies all the reconfiguration subscribers of the on-chain
    /// configurations at the specified version.
    fn notify_reconfiguration_subscribers(&mut self, version: Version) -> Result<(), Error> {
        if self.reconfig_subscriptions.is_empty() {
            return Ok(()); // No reconfiguration subscribers!
        }

        let new_configs = self.read_on_chain_configs(version)?;
        for (_, reconfig_subscription) in self.reconfig_subscriptions.iter_mut() {
            reconfig_subscription.notify_subscriber_of_configs(version, new_configs.clone())?;
        }

        Ok(())
    }

    /// Fetches the configs on-chain at the specified version.
    /// Note: We cannot assume that all configs will exist on-chain. As such, we
    /// must fetch each resource one at a time. Reconfig subscribers must be able
    /// to handle on-chain configs not existing in a reconfiguration notification.
    fn read_on_chain_configs(
        &self,
        version: Version,
    ) -> Result<OnChainConfigPayload<DbBackedOnChainConfig>, Error> {
        let db_state_view = &self
            .storage
            .read()
            .reader
            .state_view_at_version(Some(version))
            .map_err(|error| {
                Error::UnexpectedErrorEncountered(format!(
                    "Failed to create account state view {:?}",
                    error
                ))
            })?;
        let epoch = ConfigurationResource::fetch_config(&db_state_view)
            .ok_or_else(|| {
                Error::UnexpectedErrorEncountered("Configuration resource does not exist!".into())
            })?
            .epoch();

        // Return the new on-chain config payload (containing all found configs at this version).
        Ok(OnChainConfigPayload::new(
            epoch,
            DbBackedOnChainConfig::new(self.storage.read().reader.clone(), version),
        ))
    }
}

impl EventNotificationSender for EventSubscriptionService {
    fn notify_events(&mut self, version: Version, events: Vec<ContractEvent>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(()); // No events!
        }

        // Notify event subscribers and check if a reconfiguration event was processed
        let reconfig_event_processed = self.notify_event_subscribers(version, events)?;

        // If a reconfiguration event was found, also notify the reconfig subscribers
        // of the new configuration values.
        if reconfig_event_processed {
            self.notify_reconfiguration_subscribers(version)
        } else {
            Ok(())
        }
    }

    fn notify_initial_configs(&mut self, version: Version) -> Result<(), Error> {
        self.notify_reconfiguration_subscribers(version)
    }
}

/// A unique ID used to identify each subscription.
type SubscriptionId = u64;

/// A single event subscription, holding the subscription identifier, channel to
/// send the corresponding notifications and a buffer to hold pending events.
#[derive(Debug)]
struct EventSubscription {
    pub event_buffer: Vec<ContractEvent>,
    pub notification_sender: aptos_channels::aptos_channel::Sender<(), EventNotification>,
}

impl EventSubscription {
    fn buffer_event(&mut self, event: ContractEvent) {
        self.event_buffer.push(event)
    }

    fn notify_subscriber_of_events(&mut self, version: Version) -> Result<(), Error> {
        let event_notification = EventNotification {
            subscribed_events: self.event_buffer.drain(..).collect(),
            version,
        };

        self.notification_sender
            .push((), event_notification)
            .map_err(|error| Error::UnexpectedErrorEncountered(format!("{:?}", error)))
    }
}

/// A single reconfig subscription, holding the channel to send the
/// corresponding notifications.
struct ReconfigSubscription {
    pub notification_sender:
        aptos_channels::aptos_channel::Sender<(), ReconfigNotification<DbBackedOnChainConfig>>,
}

impl ReconfigSubscription {
    fn notify_subscriber_of_configs(
        &mut self,
        version: Version,
        on_chain_configs: OnChainConfigPayload<DbBackedOnChainConfig>,
    ) -> Result<(), Error> {
        let reconfig_notification = ReconfigNotification {
            version,
            on_chain_configs,
        };

        self.notification_sender
            .push((), reconfig_notification)
            .map_err(|error| Error::UnexpectedErrorEncountered(format!("{:?}", error)))
    }
}

#[derive(Clone)]
pub struct DbBackedOnChainConfig {
    pub reader: Arc<dyn DbReader>,
    pub version: Version,
}

impl DbBackedOnChainConfig {
    pub fn new(reader: Arc<dyn DbReader>, version: Version) -> Self {
        Self { reader, version }
    }
}

impl OnChainConfigProvider for DbBackedOnChainConfig {
    fn get<T: OnChainConfig>(&self) -> Result<T> {
        let bytes = self
            .reader
            .get_state_value_by_version(&StateKey::on_chain_config::<T>()?, self.version)?
            .ok_or_else(|| {
                anyhow!(
                    "no config {} found in aptos root account state",
                    T::CONFIG_ID
                )
            })?
            .bytes()
            .clone();

        T::deserialize_into_config(&bytes)
    }
}

impl fmt::Debug for DbBackedOnChainConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DbBackedOnChainConfig {{ version: {:?} }}", self.version)
    }
}

/// A notification for events.
#[derive(Debug)]
pub struct EventNotification {
    pub version: Version,
    pub subscribed_events: Vec<ContractEvent>,
}

/// A notification for reconfigurations.
#[derive(Debug)]
pub struct ReconfigNotification<P: OnChainConfigProvider> {
    pub version: Version,
    pub on_chain_configs: OnChainConfigPayload<P>,
}

/// A subscription listener for on-chain events.
pub type EventNotificationListener = NotificationListener<EventNotification>;

/// A subscription listener for reconfigurations.
pub type ReconfigNotificationListener<P> = NotificationListener<ReconfigNotification<P>>;

/// The component responsible for listening to subscription notifications.
#[derive(Debug)]
pub struct NotificationListener<T> {
    pub notification_receiver: aptos_channels::aptos_channel::Receiver<(), T>,
}

impl<T> Stream for NotificationListener<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().notification_receiver).poll_next(cx)
    }
}

impl<T> FusedStream for NotificationListener<T> {
    fn is_terminated(&self) -> bool {
        self.notification_receiver.is_terminated()
    }
}
