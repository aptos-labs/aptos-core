use std::{pin::Pin, task::{Context, Poll}};

use futures::{Future, FutureExt};
use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot},
};

/// A shutdown group allows shutting down multiple async running units and
/// await acknowledgement with one method call.
#[derive(Debug)]
pub(super) struct ShutdownGroup {
    notify_shutdown: broadcast::Sender<mpsc::Sender<()>>,
    shutdown_complete_tx: mpsc::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
}

impl ShutdownGroup {

    /// Creates a new [ShutdownGroup] from which [ShutdownHandle]s and [Shutdown]
    /// listeners can be derived.
    pub fn new() -> Self {
        let (notify_shutdown, _) = broadcast::channel(1);
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);
        Self {
            notify_shutdown,
            shutdown_complete_tx,
            shutdown_complete_rx,
        }
    }

    /// Creates and returns a new pair of [ShutdownHandle] and [Shutdown] listener.
    pub fn new_child(&self) -> (ShutdownHandle, Shutdown) {
        let (child_notify_tx, child_notify_rx) = oneshot::channel();
        (
            ShutdownHandle {
                notify_tx: child_notify_tx,
            },
            Shutdown {
                is_shutdown: false,
                global_notify: self.notify_shutdown.subscribe(),
                single_notify: child_notify_rx,
            },
        )
    }

    /// Sends the shutdown signal to all child listeners and wait for them to exit.
    pub async fn shutdown(self) {
        let Self {
            notify_shutdown,
            shutdown_complete_tx,
            mut shutdown_complete_rx,
        } = self;

        let _ = notify_shutdown.send(shutdown_complete_tx);
        drop(notify_shutdown);
        let _ = shutdown_complete_rx.recv().await;
    }
}

/// A handle with which shutdown signal can be triggered.
pub struct ShutdownHandle {
    notify_tx: oneshot::Sender<oneshot::Sender<()>>,
}

impl ShutdownHandle {
    pub(super) async fn shutdown(self) {
        let Self { notify_tx } = self;

        let (ack_tx, ack_rx) = oneshot::channel();
        if let Ok(()) = notify_tx.send(ack_tx) {
            let _ = ack_rx.await;
        }
    }
}

// TODO: introduce type alias when feature is stable
// pub type TShutdown = impl Future + Send + Unpin;

/// [Shutdown] listens to the shutdown signal from 
/// the corresponding [ShutdownHandle]
pub struct Shutdown {
    is_shutdown: bool,
    global_notify: broadcast::Receiver<mpsc::Sender<()>>,
    single_notify: oneshot::Receiver<oneshot::Sender<()>>,
}

impl Shutdown {
    pub(crate) fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    pub(crate) async fn recv(&mut self) {
        if self.is_shutdown {
            return;
        }

        select! {
            res = self.global_notify.recv() => {
                if let Ok(ack_tx) = res {
                    drop(ack_tx);
                }
            },
            res = &mut self.single_notify => {
                if let Ok(ack_tx) = res {
                    drop(ack_tx);
                }
            }
        }

        self.is_shutdown = true;
    }
}

impl Future for Shutdown {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Box::pin(self.recv()).poll_unpin(cx)
    }
}

impl Unpin for Shutdown {}