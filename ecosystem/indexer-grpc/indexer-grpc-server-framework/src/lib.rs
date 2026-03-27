// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{Context, Result};
#[cfg(target_os = "linux")]
use aptos_system_utils::profiling::start_cpu_profiling;
use axum::{
    body::Body,
    extract::State,
    http::{Response as HttpResponse, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use backtrace::Backtrace;
use clap::Parser;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use opentelemetry::trace::TracerProvider;
use prometheus::{Encoder, TextEncoder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{panic::PanicHookInfo, path::PathBuf, process};
use tracing::error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

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

    async fn status_page(&self) -> Result<Response> {
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

    async fn status_page(&self) -> Result<Response> {
        Ok((StatusCode::NOT_FOUND, "Status page is not found.").into_response())
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
    // This is a workaround until https://github.com/aptos-labs/aptos-core/issues/2038 is resolved.
    eprintln!("{}", crash_info);
    // Kill the process
    process::exit(12);
}

/// Set up logging for the server. By default we don't set a writer, in which case it
/// just logs to stdout. This can be overridden using the `make_writer` parameter.
/// This can be helpful for custom logging, e.g. logging to different files based on
/// the origin of the logging.
///
/// When `OTEL_EXPORTER_OTLP_ENDPOINT` is set, an OpenTelemetry tracing layer is
/// added that exports spans to the configured OTLP collector (e.g. Jaeger). The
/// service name is read from `OTEL_SERVICE_NAME`.
pub fn setup_logging(make_writer: Option<Box<dyn Fn() -> Box<dyn std::io::Write> + Send + Sync>>) {
    // Apply EnvFilter only to the fmt layer so RUST_LOG controls log verbosity
    // without suppressing spans from the OpenTelemetry exporter.
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let otel_layer = try_init_otel_layer();
    let registry = tracing_subscriber::registry().with(otel_layer);

    match make_writer {
        Some(w) => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(false)
                .with_thread_names(true)
                .with_writer(w)
                .with_filter(env_filter);
            registry.with(fmt_layer).init();
        },
        None => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(false)
                .with_thread_names(true)
                .with_filter(env_filter);
            registry.with(fmt_layer).init();
        },
    }
}

/// Attempt to initialise an OpenTelemetry OTLP exporter layer. Returns `None`
/// when `OTEL_EXPORTER_OTLP_ENDPOINT` is not set or the pipeline fails to
/// initialise, so callers can treat OTLP export as opt-in.
fn try_init_otel_layer() -> Option<
    tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry_sdk::trace::Tracer,
    >,
> {
    if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_err() {
        return None;
    }

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .ok()?;

    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "unknown".to_string());

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name),
        ]))
        .build();

    let tracer = provider.tracer("indexer-grpc");
    // Keep the provider alive for the lifetime of the process so spans are flushed.
    std::mem::forget(provider);

    Some(tracing_opentelemetry::layer().with_tracer(tracer))
}

/// Register readiness and liveness probes and set up metrics endpoint.
async fn register_probes_and_metrics_handler<C>(config: GenericConfig<C>, port: u16)
where
    C: RunnableConfig,
{
    let app = Router::new()
        .route("/readiness", get(readiness_handler))
        .route("/metrics", get(metrics_handler))
        .route("/", get(status_handler::<C>))
        .with_state(config);

    #[cfg(target_os = "linux")]
    let app = app.route("/profilez", get(profilez_handler));

    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port))
        .await
        .expect("Failed to bind probes and metrics listener");
    axum::serve(listener, app)
        .await
        .expect("Probes and metrics server exited unexpectedly");
}

async fn readiness_handler() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ready")
}

async fn metrics_handler() -> Response {
    // Metrics encoding.
    let metrics = aptos_metrics_core::gather();
    let mut encode_buffer = vec![];
    let encoder = TextEncoder::new();
    // If metrics encoding fails, we want to panic and crash the process.
    encoder
        .encode(&metrics, &mut encode_buffer)
        .context("Failed to encode metrics")
        .unwrap();

    HttpResponse::builder()
        .header("Content-Type", "text/plain")
        .body(Body::from(encode_buffer))
        .expect("Failed to build metrics response")
}

async fn status_handler<C>(State(config): State<GenericConfig<C>>) -> Response
where
    C: RunnableConfig,
{
    match config.status_page().await {
        Ok(response) => response,
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to render status page: {err:#}"),
        )
            .into_response(),
    }
}

#[cfg(target_os = "linux")]
async fn profilez_handler() -> Response {
    // TODO(grao): Consider make the parameters configurable.
    match start_cpu_profiling(10, 99, false).await {
        Ok(body) => {
            match HttpResponse::builder()
                .header("Content-Length", body.len().to_string())
                .header("Content-Disposition", "inline")
                .header("Content-Type", "image/svg+xml")
                .body(Body::from(body))
            {
                Ok(response) => response,
                Err(err) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Profiling failed: {err:?}."),
                )
                    .into_response(),
            }
        },
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Profiling failed: {err:?}."),
        )
            .into_response(),
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
