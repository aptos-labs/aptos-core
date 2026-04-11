// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::{marker::PhantomData, sync::Mutex};
use tokio::{runtime::Handle, task::JoinHandle};

/// A scope for spawning blocking tasks on a tokio runtime.
/// All spawned tasks are joined when [`scope_blocking`] returns.
pub struct BlockingScope<'env> {
    handle: Handle,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    _marker: PhantomData<&'env mut &'env ()>,
}

impl<'env> BlockingScope<'env> {
    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'env,
    {
        // SAFETY: All tasks are joined in `scope_blocking` before 'env expires,
        // maintaining the same invariant as rayon::Scope.
        let f: Box<dyn FnOnce() + Send + 'static> = unsafe {
            std::mem::transmute::<Box<dyn FnOnce() + Send + 'env>, Box<dyn FnOnce() + Send + 'static>>(
                Box::new(f),
            )
        };
        self.tasks
            .lock()
            .unwrap()
            .push(self.handle.spawn_blocking(move || f()));
    }
}

/// Runs `op`, which may spawn blocking tasks via [`BlockingScope::spawn`],
/// then joins all spawned tasks before returning. Analogous to `rayon::scope`.
pub fn scope_blocking<'env, OP, R>(handle: &Handle, op: OP) -> R
where
    OP: FnOnce(&BlockingScope<'env>) -> R,
{
    let scope = BlockingScope {
        handle: handle.clone(),
        tasks: Mutex::new(Vec::new()),
        _marker: PhantomData,
    };
    let result = op(&scope);
    handle.block_on(async {
        for task in scope.tasks.into_inner().unwrap() {
            task.await.unwrap();
        }
    });
    result
}

/// Runs a single closure as a blocking task on the given tokio runtime and
/// returns its result. Analogous to `rayon::ThreadPool::install`.
pub fn install_blocking<'env, F, R>(handle: &Handle, f: F) -> R
where
    F: FnOnce() -> R + Send + 'env,
    R: Send + 'static,
{
    // SAFETY: The task is joined before returning, so 'env remains valid.
    let f: Box<dyn FnOnce() -> R + Send + 'static> = unsafe {
        std::mem::transmute::<
            Box<dyn FnOnce() -> R + Send + 'env>,
            Box<dyn FnOnce() -> R + Send + 'static>,
        >(Box::new(f))
    };
    handle.block_on(async { tokio::task::spawn_blocking(move || f()).await.unwrap() })
}
