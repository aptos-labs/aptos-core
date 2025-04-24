// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, future::Future, task, time::Duration};
use tokio::time::{sleep_until, Instant};

pub trait TimerService: Send + Sync {
    type Event;

    fn schedule(&mut self, duration: Duration, event: Self::Event);
    fn tick(&mut self) -> impl Future<Output = Self::Event> + Send;
}

pub struct LocalTimerService<E> {
    deadlines: BTreeMap<Instant, E>,
}

impl<E> LocalTimerService<E> {
    pub fn new() -> Self {
        LocalTimerService {
            deadlines: BTreeMap::new(),
        }
    }
}

impl<E: Send + Sync> TimerService for LocalTimerService<E> {
    type Event = E;

    fn schedule(&mut self, duration: Duration, event: E) {
        self.deadlines.insert(Instant::now() + duration, event);
    }

    async fn tick(&mut self) -> E {
        let next_deadline = self.deadlines.iter().next().map(|(deadline, _)| *deadline);

        match next_deadline {
            Some(next_deadline) => {
                sleep_until(next_deadline).await;
                let (_, event) = self.deadlines.remove_entry(&next_deadline).unwrap();
                event
            },
            None => {
                NeverReturn {}.await;
                unreachable!()
            },
        }
    }
}

#[derive(Clone)]
pub struct InjectedTimerService<U, I> {
    underlying: U,
    injection: I,
}

pub trait TimerInjection<E>: Fn(Duration, E) -> (Duration, E) + Send + Sync + 'static {}

impl<I, E> TimerInjection<E> for I where I: Fn(Duration, E) -> (Duration, E) + Send + Sync + 'static {}

pub fn clock_skew_injection<E>(clock_speed: f64) -> impl TimerInjection<E> {
    move |duration, event| {
        (
            Duration::from_secs_f64(duration.as_secs_f64() / clock_speed),
            event,
        )
    }
}

impl<U, I> InjectedTimerService<U, I>
where
    U: TimerService,
    I: TimerInjection<U::Event>,
{
    pub fn new(underlying: U, injection: I) -> Self {
        InjectedTimerService {
            underlying,
            injection,
        }
    }
}

impl<E: Send + Sync, I> InjectedTimerService<LocalTimerService<E>, I>
where
    I: TimerInjection<E>,
{
    pub fn local(injection: I) -> Self {
        InjectedTimerService::new(LocalTimerService::new(), injection)
    }
}

impl<U, I> TimerService for InjectedTimerService<U, I>
where
    U: TimerService + Send,
    I: TimerInjection<U::Event>,
{
    type Event = U::Event;

    fn schedule(&mut self, duration: Duration, event: Self::Event) {
        let (duration, event) = (self.injection)(duration, event);
        self.underlying.schedule(duration, event);
    }

    async fn tick(&mut self) -> Self::Event {
        self.underlying.tick().await
    }
}

/// NeverExpire is a future that never unblocks
pub struct NeverReturn {}

impl Future for NeverReturn {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        task::Poll::Pending
    }
}
