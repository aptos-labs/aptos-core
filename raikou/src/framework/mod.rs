use crate::framework::context::{Context, Event, SimpleContext};
use std::{future::Future, sync::Arc};

pub mod context;
pub mod network;
pub mod timer;

pub type NodeId = usize;

// Trait alias.
pub trait ContextFor<P: Protocol + ?Sized>:
    Context<Message = P::Message, TimerEvent = P::TimerEvent>
{
}

impl<P, Ctx> ContextFor<P> for Ctx
where
    P: Protocol + ?Sized,
    Ctx: Context<Message = P::Message, TimerEvent = P::TimerEvent>,
{
}

pub trait Protocol: Send + Sync {
    type Message: Send + Sync;
    type TimerEvent: Send + Sync;

    fn start_handler<Ctx>(&mut self, ctx: &mut Ctx) -> impl Future<Output = ()> + Send
    where
        Ctx: ContextFor<Self>;

    fn message_handler<Ctx>(
        &mut self,
        ctx: &mut Ctx,
        from: NodeId,
        message: Self::Message,
    ) -> impl Future<Output = ()> + Send
    where
        Ctx: ContextFor<Self>;

    fn timer_event_handler<Ctx>(
        &mut self,
        ctx: &mut Ctx,
        event: Self::TimerEvent,
    ) -> impl Future<Output = ()> + Send
    where
        Ctx: ContextFor<Self>;

    fn condition_handler<Ctx>(&mut self, ctx: &mut Ctx) -> impl Future<Output = ()> + Send
    where
        Ctx: ContextFor<Self>;

    fn run_ctx<Ctx>(
        protocol: &tokio::sync::Mutex<Self>,
        ctx: &mut Ctx,
    ) -> impl Future<Output = ()> + Send
    where
        Ctx: ContextFor<Self>,
    {
        async move {
            protocol.lock().await.start_handler(ctx).await;

            while !ctx.halted() {
                // Run the condition handlers
                protocol.lock().await.condition_handler(ctx).await;

                // Listen for incoming events
                match ctx.next_event().await {
                    Event::Message(from, message) => {
                        protocol
                            .lock()
                            .await
                            .message_handler(ctx, from, message)
                            .await;
                    },
                    Event::Timer(event) => {
                        protocol.lock().await.timer_event_handler(ctx, event).await;
                    },
                }
            }
        }
    }

    fn run<NS, TS>(
        protocol: &tokio::sync::Mutex<Self>,
        node_id: NodeId,
        network_service: NS,
        timer: TS,
    ) -> impl Future<Output = ()> + Send
    where
        NS: network::NetworkService<Message = Self::Message>,
        TS: timer::TimerService<Event = Self::TimerEvent>,
    {
        async move {
            let mut context = SimpleContext::new(node_id, network_service, timer);
            Protocol::run_ctx(protocol, &mut context).await;
        }
    }
}

/// Should be used as follows:
/// ```
/// impl Protocol for MyProtocol {
///     type Message = MyMessageType;
///     type TimerEvent = MyTimerEventType;
///
///     protocol! {
///         // These lines need to be written as below at the beginning.
///         self: self;
///         ctx: ctx;
///
///         // Handlers (see syntax below)
///         // Note that each handler must be followed by a semicolon.
///
///         // Start handler executes once, upon starting the node.
///         upon start {
///             ...
///         };
///
///         // Handlers for messages received from other nodes.
///         upon receive [MESSAGE_PATTERN] from [SENDER_PATTERN] {
///            ...
///         };
///
///         // Handlers for timer events.
///         upon timer [TIMER_PATTERN] {
///             ...
///         };
///
///         // Conditional handlers are executed in a loop until the condition is false
///         // each time after receiving a message or a timer event.
///         upon [CONDITION] {
///             ...
///         };
///
///         // You can specify a variable number of conditional handlers at once.
///         for [VAR in RANGE]
///         upon [CONDITION] {
///             ...
///         };
///     }
/// }
/// ```
#[macro_export]
macro_rules! protocol {
    (
        /// $self must be "self" and nothing else.
        /// $ctx is the name for the context variable passed to the handlers.
        self: $self:ident;
        ctx: $ctx:ident;

        $(
            // Start handler executes once, upon starting the node.
            $(upon start $($start_label:lifetime:)? $start_handler:block)?
            // Handlers for messages received from other nodes.
            $(upon receive [$msg_pat:pat] from $(node)? [$from_pat:pat] $(if [$msg_cond:expr])? $($msg_label:lifetime:)? $msg_handler:block)?
            // Handlers for timer events.
            $(upon timer $(event)? [$timer_pat:pat] $(if [$timer_cond:expr])?  $($timer_label:lifetime:)? $timer_handler:block)?
            // Conditional handlers are executed in a loop until the condition is false
            // each time after a receiving a message or a timer event.
            $(upon [$cond:expr] $(if [$cond_cond:expr])? $($cond_label:lifetime:)? $cond_handler:block)?
            $(for [$cond_loop_var:ident in $cond_loop_range:expr] upon [$cond_loop_cond:expr] $($cond_loop_label:lifetime:)? $cond_loop_handler:block)?
            // Due to some limitations of the declarative macro system,
            // every handler must be followed by a semicolon.
            ;
        )*
    ) => {
        async fn start_handler<Ctx>(&mut $self, $ctx: &mut Ctx)
        where
            Ctx: crate::framework::ContextFor<Self>,
        {
            let _ = $ctx;  // suppress unused variable warning
            $(
                $(
                    $($start_label:)? {
                        $start_handler;
                    }
                )?
            )*
        }

        async fn message_handler<Ctx>(&mut $self, $ctx: &mut Ctx, from: NodeId, message: Self::Message)
        where
            Ctx: crate::framework::ContextFor<Self>
        {
            let _ = $ctx;  // suppress unused variable warning
            match (from, message) {
                $(
                    $(
                        ($from_pat, $msg_pat) $(if $msg_cond)? => $($msg_label:)? {
                            $msg_handler;
                        },
                    )?
                )*
            }
        }

        async fn timer_event_handler<Ctx>(&mut $self, $ctx: &mut Ctx, event: Self::TimerEvent)
        where
            Ctx: crate::framework::ContextFor<Self>
        {
            let _ = $ctx;  // suppress unused variable warning
            match event {
                $(
                    $(
                        $timer_pat $(if $timer_cond)? => $($timer_label:)? {
                            $timer_handler;
                        },
                    )?
                )*
            }
        }

        async fn condition_handler<Ctx>(&mut $self, $ctx: &mut Ctx)
        where
            Ctx: crate::framework::ContextFor<Self>
        {
            let _ = $ctx;  // suppress unused variable warning
            let mut n_hits: u64 = 0;
            let mut was_hit = true;

            while was_hit && !$ctx.halted() {
                was_hit = false;
                let mut hit = || {
                    was_hit = true;
                    n_hits += 1;
                    if n_hits % 100_000 == 0 {
                        log::warn!("Condition handler looped {} times. \
                        Possible infinite loop.", n_hits);
                    }
                };
                let _ = &mut hit;  // suppress unused variable warning

                $(
                    $(
                        while !$ctx.halted() && $cond $(&& $cond_cond)? {
                            hit();
                            $($cond_label:)? {
                                $cond_handler;
                            }
                        }
                    )?
                )*
                $(
                    $(
                        for $cond_loop_var in $cond_loop_range {
                            while !$ctx.halted() && $cond_loop_cond {
                                hit();
                                $($cond_loop_label:)? {
                                    $cond_loop_handler;
                                }
                            }
                        }
                    )?
                )*
            }
        }
    }
}
