// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
#[cfg(target_os = "linux")]
use velor_system_utils::profiling::start_cpu_profiling;
use backtrace::Backtrace;
use clap::Parser;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use prometheus::{Encoder, TextEncoder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::convert::Infallible;
use std::{panic::PanicHookInfo, path::PathBuf, process};
use tracing::error;
use tracing_subscriber::EnvFilter;
use warp::{http::Response, reply::Reply, Filter};

/// ServerArgs bootstraps a server with all common pieces. And then triggers the run method for
/// the specific service.
#[derive(Parser)]
pub struct ServerArgs {
    #[clap(short, long, value_parser)]
    pub config_path: PathBuf,
}

impl ServerArgs {
    pub async fn run<C>(&self) -> Result<()>
    where
        C: RunnableConfig,
    {
        // Set up the server.
        setup_logging(None);
        setup_panic_handler();
        let config = load::<GenericConfig<C>>(&self.config_path)?;
        config
            .validate()
            .context("Config did not pass validation")?;
        run_server_with_config(config).await
    }
}

pub async fn run_server_with_config<C>(config: GenericConfig<C>) -> Result<()>
where
    C: RunnableConfig,
{
    let health_port = config.health_check_port;
    // Start liveness and readiness probes.
    let config_clone = config.clone();
    let task_handler = tokio::spawn(async move {
        register_probes_and_metrics_handler(config_clone, health_port).await;
        anyhow::Ok(())
    });
    let main_task_handler =
        tokio::spawn(async move { config.run().await.expect("task should exit with Ok.") });
    tokio::select! {
        res = task_handler => {
            if let Err(e) = res {
                error!("Probes and metrics handler panicked or was shutdown: {:?}", e);
                process::exit(1);
            } else {
                panic!("Probes and metrics handler exited unexpectedly");
            }
        },
        res = main_task_handler => {
            if let Err(e) = res {
                error!("Main task panicked or was shutdown: {:?}", e);
                process::exit(1);
            } else {
                panic!("Main task exited unexpectedly");
            }
        },
    }
}

#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct GenericConfig<T> {
    // Shared configuration among all services.
    pub health_check_port: u16,

    // Specific configuration for each service.
    pub server_config: T,
}

#[async_trait::async_trait]
impl<T> RunnableConfig for GenericConfig<T>
where
    T: RunnableConfig,
{
    fn validate(&self) -> Result<()> {
        self.server_config.validate()
    }

    async fn run(&self) -> Result<()> {
        self.server_config.run().await
    }

    fn get_server_name(&self) -> String {
        self.server_config.get_server_name()
    }

    async fn status_page(&self) -> Result<warp::reply::Response, warp::Rejection> {
        self.server_config.status_page().await
    }
}

/// RunnableConfig is a trait that all services must implement for their configuration.
#[async_trait::async_trait]
pub trait RunnableConfig: Clone + DeserializeOwned + Send + Sync + 'static {
    // Validate the config.
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    // Run something based on the config.
    async fn run(&self) -> Result<()>;

    // Get the server name.
    fn get_server_name(&self) -> String;

    async fn status_page(&self) -> Result<warp::reply::Response, warp::Rejection> {
        Ok("Status page is not found.".into_response())
    }
}

/// Parse a yaml file into a struct.
pub fn load<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Result<T> {
    Figment::new()
        .merge(Yaml::file(path))
        .merge(Env::raw().split("__"))
        .extract()
        .map_err(anyhow::Error::msg)
}

#[derive(Debug, Serialize)]
pub struct CrashInfo {
    details: String,
    backtrace: String,
}

/// Invoke to ensure process exits on a thread panic.
///
/// Tokio's default behavior is to catch panics and ignore them. Invoking this function will
/// ensure that all subsequent thread panics (even Tokio threads) will report the
/// details/backtrace and then exit.
pub fn setup_panic_handler() {
    std::panic::set_hook(Box::new(move |pi: &PanicHookInfo<'_>| {
        handle_panic(pi);
    }));
}

// Formats and logs panic information
fn handle_panic(panic_info: &PanicHookInfo<'_>) {
    // The Display formatter for a PanicHookInfo contains the message, payload and location.
    let details = format!("{}", panic_info);
    let backtrace = format!("{:#?}", Backtrace::new());
    let info = CrashInfo { details, backtrace };
    let crash_info = toml::to_string_pretty(&info).unwrap();
    error!("{}", crash_info);
    // TODO / HACK ALARM: Write crash info synchronously via eprintln! to ensure it is written before the process exits which error! doesn't guarantee.
    // This is a workaround until https://github.com/velor-chain/velor-core/issues/2038 is resolved.
    eprintln!("{}", crash_info);
    // Kill the process
    process::exit(12);
}

/// Set up logging for the server. By default we don't set a writer, in which case it
/// just logs to stdout. This can be overridden using the `make_writer` parameter.
/// This can be helpful for custom logging, e.g. logging to different files based on
/// the origin of the logging.
pub fn setup_logging(make_writer: Option<Box<dyn Fn() -> Box<dyn std::io::Write> + Send + Sync>>) {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .with_thread_names(true)
        .with_env_filter(env_filter);

    match make_writer {
        Some(w) => subscriber.with_writer(w).init(),
        None => subscriber.init(),
    }
}

/// Register readiness and liveness probes and set up metrics endpoint.
async fn register_probes_and_metrics_handler<C>(config: GenericConfig<C>, port: u16)
where
    C: RunnableConfig,
{
    let readiness = warp::path("readiness")
        .map(move || warp::reply::with_status("ready", warp::http::StatusCode::OK));

    let metrics_endpoint = warp::path("metrics").map(|| {
        // Metrics encoding.
        let metrics = velor_metrics_core::gather();
        let mut encode_buffer = vec![];
        let encoder = TextEncoder::new();
        // If metrics encoding fails, we want to panic and crash the process.
        encoder
            .encode(&metrics, &mut encode_buffer)
            .context("Failed to encode metrics")
            .unwrap();

        Response::builder()
            .header("Content-Type", "text/plain")
            .body(encode_buffer)
    });

    let status_endpoint = warp::path::end().and_then(move || {
        let config = config.clone();
        async move { config.status_page().await }
    });

    if cfg!(target_os = "linux") {
        #[cfg(target_os = "linux")]
        let profilez = warp::path("profilez").and_then(|| async move {
            // TODO(grao): Consider make the parameters configurable.
            Ok::<_, Infallible>(match start_cpu_profiling(10, 99, false).await {
                Ok(body) => {
                    let response = Response::builder()
                        .header("Content-Length", body.len())
                        .header("Content-Disposition", "inline")
                        .header("Content-Type", "image/svg+xml")
                        .body(body);

                    match response {
                        Ok(res) => warp::reply::with_status(res, warp::http::StatusCode::OK),
                        Err(e) => warp::reply::with_status(
                            Response::new(format!("Profiling failed: {e:?}.").as_bytes().to_vec()),
                            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                        ),
                    }
                },
                Err(e) => warp::reply::with_status(
                    Response::new(format!("Profiling failed: {e:?}.").as_bytes().to_vec()),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ),
            })
        });
        #[cfg(target_os = "linux")]
        warp::serve(
            readiness
                .or(metrics_endpoint)
                .or(status_endpoint)
                .or(profilez),
        )
        .run(([0, 0, 0, 0], port))
        .await;
    } else {
        warp::serve(readiness.or(metrics_endpoint).or(status_endpoint))
            .run(([0, 0, 0, 0], port))
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write};
    use tempfile::tempdir;

    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct TestConfig {
        test: u32,
        test_name: String,
    }

    #[async_trait::async_trait]
    impl RunnableConfig for TestConfig {
        async fn run(&self) -> Result<()> {
            assert_eq!(self.test, 123);
            assert_eq!(self.test_name, "test");
            Ok(())
        }

        fn get_server_name(&self) -> String {
            self.test_name.clone()
        }
    }

    #[test]
    fn test_random_config_creation() {
        let dir = tempdir().expect("tempdir failure");

        let file_path = dir.path().join("testing_yaml.yaml");
        let mut file = File::create(&file_path).expect("create failure");
        let raw_yaml_content = r#"
            health_check_port: 12345
            server_config:
                test: 123
                test_name: "test"
        "#;
        writeln!(file, "{}", raw_yaml_content).expect("write_all failure");

        let config = load::<GenericConfig<TestConfig>>(&file_path).unwrap();
        assert_eq!(config.health_check_port, 12345);
        assert_eq!(config.server_config.test, 123);
        assert_eq!(config.server_config.test_name, "test");
    }

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        ServerArgs::command().debug_assert()
    }
}
