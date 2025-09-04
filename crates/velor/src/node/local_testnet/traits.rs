// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::health_checker::HealthChecker;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::{collections::HashSet, fmt::Debug};
use tracing::warn;

#[async_trait]
pub trait ServiceManager: Debug + Send + Sync + 'static {
    /// Pretty name that we will show to the user for updates about this service.
    fn get_name(&self) -> String;

    /// This is called before the service is run. This is a good place to do any
    /// setup that needs to be done before the service is run.
    async fn pre_run(&self) -> Result<()> {
        Ok(())
    }

    /// All services should expose some way to check if they are healthy. This function
    /// returns HealthCheckers, a struct that serves this purpose, that later services
    /// can use to make sure prerequisite services have started. These are also used
    /// by the "ready server", a server that exposes a unified endpoint for checking
    /// if all services are ready.
    fn get_health_checkers(&self) -> HashSet<HealthChecker>;

    /// Whereas get_health_checkers returns healthchecks that other downstream services
    /// can use, this should return health checkers for services that this service is
    /// waiting to start.
    //
    // Note: If we were using an object oriented language, we'd just make the
    // constructor of the superclass require a vec of health checkers. Unfortunately
    // we can't do that here, hence this runaround where the trait implementer must
    // individually handle accepting health checkers and exposing them here. Similarly,
    // if we could make this function private we would, but we can't since right now
    // all functions in a trait must be pub.
    fn get_prerequisite_health_checkers(&self) -> HashSet<&HealthChecker>;

    /// This is the function we use from the outside to start the service. It makes
    /// sure all the prerequisite services have started and then calls the inner
    /// function to run the service. The user should never need to override this
    /// implementation.
    async fn run(self: Box<Self>) -> Result<()> {
        // We make a new function here so that each task waits for its prereqs within
        // its own run function. This way we can start each service in any order.
        let name = self.get_name();
        let name_clone = name.to_string();
        for health_checker in self.get_prerequisite_health_checkers() {
            health_checker
                .wait(Some(&self.get_name()))
                .await
                .context("Prerequisite service did not start up successfully")?;
        }
        self.run_service()
            .await
            .context("Service ended with an error")?;
        warn!(
            "Service {} ended unexpectedly without any error",
            name_clone
        );
        Ok(())
    }

    /// The ServiceManager may return PostHealthySteps. The tool will run these after
    /// the service is started. See `PostHealthyStep` for more information.
    //
    // You might ask, why not just have a `post_healthy` function? The problem is we
    // want `run` to take `self` so the implementer doesn't have to worry about making
    // their config Clone, so after that point the ServiceManager won't exist. Hence
    // this model.
    fn get_post_healthy_steps(&self) -> Vec<Box<dyn PostHealthyStep>> {
        vec![]
    }

    /// The ServiceManager may return ShutdownSteps. The tool will run these on shutdown.
    /// This is best effort, there is nothing we can do if part of the code aborts or
    /// the process receives something like SIGKILL.
    ///
    /// See `ShutdownStep` for more information.
    fn get_shutdown_steps(&self) -> Vec<Box<dyn ShutdownStep>> {
        vec![]
    }

    /// This function is responsible for running the service. It should return an error
    /// if the service ends unexpectedly. It gets called by `run`.
    async fn run_service(self: Box<Self>) -> Result<()>;
}

/// If a service wants to do something after it is healthy, it can define a struct,
/// implement this trait for it, and return an instance of it.
///
/// For more information see `get_post_healthy_steps` in `ServiceManager`.
#[async_trait]
pub trait PostHealthyStep: Debug + Send + Sync + 'static {
    async fn run(self: Box<Self>) -> Result<()>;
}

/// If a service wants to do something on shutdown, it can define a struct,
/// implement this trait for it, and return an instance of it.
///
/// For more information see `get_shutdown_steps` in `ServiceManager`.
#[async_trait]
pub trait ShutdownStep: Debug + Send + Sync + 'static {
    async fn run(self: Box<Self>) -> Result<()>;
}
