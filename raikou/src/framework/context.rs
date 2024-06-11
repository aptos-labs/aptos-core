use crate::framework::{network::NetworkService, timer::TimerService, NodeId};
use std::{future::Future, marker::PhantomData, time::Duration};
use tokio::select;

pub enum Event<M, TE> {
    Message(NodeId, M),
    Timer(TE),
}
pub trait Context: Send + Sync {
    type Message: Clone + Send + Sync;
    type TimerEvent;

    fn node_id(&self) -> NodeId;

    fn n_nodes(&self) -> usize;

    // Unicast sends the message to a single node.
    fn unicast(&self, message: Self::Message, target: NodeId) -> impl Future<Output = ()> + Send;

    /// Multicast sends the same message to all nodes in the network.
    fn multicast(&self, message: Self::Message) -> impl Future<Output = ()> + Send {
        async move {
            for target in 0..self.n_nodes() {
                self.unicast(message.clone(), target).await;
            }
        }
    }

    fn set_timer(&mut self, duration: std::time::Duration, event: Self::TimerEvent);

    fn halt(&mut self);

    fn halted(&self) -> bool;

    fn next_event(&mut self)
        -> impl Future<Output = Event<Self::Message, Self::TimerEvent>> + Send;
}

// impl<'a, Ctx: Context> Context for &'a mut Ctx {
//     type Message = Ctx::Message;
//     type TimerEvent = Ctx::TimerEvent;
//
//     fn node_id(&self) -> NodeId {
//         (**self).node_id()
//     }
//
//     fn n_nodes(&self) -> usize {
//         (**self).n_nodes()
//     }
//
//     async fn unicast(&self, message: Self::Message, target: NodeId) {
//         (**self).unicast(message, target).await;
//     }
//
//     async fn multicast(&self, message: Self::Message) {
//         (**self).multicast(message).await;
//     }
//
//     async fn next_event(&mut self) -> Event<Self::Message, Self::TimerEvent> {
//         (**self).next_event().await
//     }
// }

pub struct SimpleContext<NS, TS> {
    id: NodeId,
    network: NS,
    timer: TS,
    halted: bool,
}

impl<NS: NetworkService, TS: TimerService> SimpleContext<NS, TS> {
    pub fn new(id: NodeId, network: NS, timer: TS) -> Self {
        SimpleContext {
            id,
            network,
            timer,
            halted: false,
        }
    }
}

impl<NS: NetworkService, TS: TimerService> Context for SimpleContext<NS, TS> {
    type Message = <NS as NetworkService>::Message;
    type TimerEvent = <TS as TimerService>::Event;

    fn node_id(&self) -> NodeId {
        self.id
    }

    fn n_nodes(&self) -> usize {
        self.network.n_nodes()
    }

    async fn unicast(&self, message: Self::Message, target: NodeId) {
        self.network.send(target, message).await;
    }

    fn set_timer(&mut self, duration: std::time::Duration, event: Self::TimerEvent) {
        self.timer.schedule(duration, event);
    }

    fn halt(&mut self) {
        self.halted = true;
    }

    fn halted(&self) -> bool {
        self.halted
    }

    async fn next_event(&mut self) -> Event<Self::Message, Self::TimerEvent> {
        select! {
            (from, message) = self.network.recv() => {
                Event::Message(from, message)
            },
            timer_event = self.timer.tick() => {
                Event::Timer(timer_event)
            }
        }
    }
}

/// A hacky way to wrap a context for basic sub-protocol functionality.
/// Passing the events from the outer protocol to the inner ones needs to be done manually.
pub struct WrappedContext<'a, Ctx, M, TE, WM, WTE> {
    inner: &'a mut Ctx,
    wrap_message: WM,
    wrap_timer_event: WTE,

    _phantom: PhantomData<(M, TE)>,
}

impl<'a, Ctx, M, TE, WM, WTE> WrappedContext<'a, Ctx, M, TE, WM, WTE>
where
    Ctx: Context,
    M: Clone + Send + Sync,
    TE: Clone + Send + Sync,
    WM: Fn(M) -> Ctx::Message + Clone + Send + Sync,
    WTE: Fn(TE) -> Ctx::TimerEvent + Clone + Send + Sync,
{
    pub fn new(inner: &'a mut Ctx, wrap_message: WM, wrap_timer_event: WTE) -> Self {
        WrappedContext {
            inner,
            wrap_message,
            wrap_timer_event,
            _phantom: PhantomData,
        }
    }
}

impl<'a, Ctx, M, TE, WM, WTE> Context for WrappedContext<'a, Ctx, M, TE, WM, WTE>
where
    Ctx: Context,
    M: Clone + Send + Sync,
    TE: Clone + Send + Sync,
    WM: Fn(M) -> Ctx::Message + Clone + Send + Sync,
    WTE: Fn(TE) -> Ctx::TimerEvent + Clone + Send + Sync,
{
    type Message = M;
    type TimerEvent = TE;

    fn node_id(&self) -> NodeId {
        self.inner.node_id()
    }

    fn n_nodes(&self) -> usize {
        self.inner.n_nodes()
    }

    async fn unicast(&self, message: Self::Message, target: NodeId) {
        self.inner
            .unicast((self.wrap_message)(message), target)
            .await;
    }

    async fn multicast(&self, message: Self::Message) {
        self.inner.multicast((self.wrap_message)(message)).await;
    }

    fn set_timer(&mut self, duration: Duration, event: Self::TimerEvent) {
        self.inner
            .set_timer(duration, (self.wrap_timer_event)(event));
    }

    fn halt(&mut self) {
        self.inner.halt();
    }

    fn halted(&self) -> bool {
        self.inner.halted()
    }

    async fn next_event(&mut self) -> Event<Self::Message, Self::TimerEvent> {
        unimplemented!();
    }
}
