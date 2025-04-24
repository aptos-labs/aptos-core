// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    any::{Any, TypeId},
    collections::{btree_map::Entry, BTreeMap},
    fmt::Debug,
    sync::Arc,
};
use tokio::sync::{mpsc, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleId(usize);

impl ModuleId {
    pub fn next(&self) -> Self {
        ModuleId(self.0 + 1)
    }

    /// Returns the first dynamic module ID.
    /// This and higher module IDs are reserved for
    pub fn first_dynamic() -> Self {
        ModuleId(1_000_000_000)
    }
}

pub trait ModuleEventTrait: Any + Debug + Send + 'static {
    fn debug_string(&self) -> String {
        format!("{:?}", self)
    }

    fn event_type_id(&self) -> TypeId {
        self.type_id()
    }

    // TODO: figure out if it's possible to add a default implementation for this method.
    fn as_any(self: Box<Self>) -> Box<dyn Any>;
}

/// Fool-proof way to match event types.
pub fn match_event_type<E: ModuleEventTrait>(event: &ModuleEvent) -> bool {
    event.event_type_id() == TypeId::of::<E>()
}

pub type ModuleEvent = Box<dyn ModuleEventTrait>;

pub struct ModuleNetwork {
    send: Arc<RwLock<BTreeMap<ModuleId, mpsc::Sender<(ModuleId, ModuleEvent)>>>>,
    next_id: ModuleId,
}

impl ModuleNetwork {
    pub fn new() -> Self {
        ModuleNetwork {
            send: Arc::new(RwLock::new(BTreeMap::new())),
            next_id: ModuleId::first_dynamic(),
        }
    }

    pub async fn register(&mut self) -> ModuleNetworkService {
        let module = self.next_id;
        self.next_id = ModuleId(module.0 + 1);
        self.register_with_id(module).await
    }

    pub async fn register_with_id(&mut self, module: ModuleId) -> ModuleNetworkService {
        match self.send.write().await.entry(module) {
            Entry::Occupied(_) => panic!("Module id {:?} already registered", module),
            Entry::Vacant(entry) => {
                let (send, recv) = tokio::sync::mpsc::channel(100);
                entry.insert(send);

                ModuleNetworkService {
                    sender: ModuleNetworkSender {
                        module_id: module,
                        send: self.send.clone(),
                    },
                    receive: recv,
                }
            },
        }
    }
}

#[derive(Clone)]
pub struct ModuleNetworkSender {
    module_id: ModuleId,
    send: Arc<RwLock<BTreeMap<ModuleId, mpsc::Sender<(ModuleId, ModuleEvent)>>>>,
}

pub struct ModuleNetworkService {
    sender: ModuleNetworkSender,
    receive: mpsc::Receiver<(ModuleId, ModuleEvent)>,
}

impl ModuleNetworkService {
    pub fn module_id(&self) -> ModuleId {
        self.sender.module_id
    }

    pub async fn notify_boxed(&self, module: ModuleId, event: ModuleEvent) {
        self.sender.notify_boxed(module, event).await;
    }

    pub async fn notify<E: ModuleEventTrait>(&self, module: ModuleId, event: E) {
        self.notify_boxed(module, Box::new(event)).await;
    }

    pub async fn recv(&mut self) -> (ModuleId, ModuleEvent) {
        self.receive.recv().await.unwrap()
    }

    pub fn new_sender(&self) -> ModuleNetworkSender {
        self.sender.clone()
    }
}

impl ModuleNetworkSender {
    pub async fn notify_boxed(&self, module: ModuleId, event: ModuleEvent) {
        // Ignore errors if the receiver is gone.
        let _res = self.send.read().await[&module]
            .send((self.module_id, event))
            .await;
    }

    pub async fn notify<E: ModuleEventTrait>(&self, module: ModuleId, event: E) {
        self.notify_boxed(module, Box::new(event)).await;
    }
}
