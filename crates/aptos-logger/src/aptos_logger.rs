// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(unexpected_cfgs)]

//! Implementation of writing logs to both local printers (e.g. stdout) and remote loggers
//! (e.g. Logstash)

use crate::{
    counters::{
        PROCESSED_STRUCT_LOG_COUNT, STRUCT_LOG_PARSE_ERROR_COUNT, STRUCT_LOG_QUEUE_ERROR_COUNT,
    },
    logger::Logger,
    sample,
    sample::SampleRate,
    telemetry_log_writer::{TelemetryLog, TelemetryLogWriter},
    Event, Filter, Key, Level, LevelFilter, Metadata, ERROR_LOG_COUNT, INFO_LOG_COUNT,
    WARN_LOG_COUNT,
};
use aptos_infallible::RwLock;
use backtrace::Backtrace;
use chrono::{SecondsFormat, Utc};
use futures::channel;
use once_cell::sync::Lazy;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    env,
    fmt::{self, Debug},
    io::{Stdout, Write},
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::{self, Arc},
    thread,
    time::Duration,
};
use strum_macros::EnumString;
use tokio::time;

const RUST_LOG: &str = "RUST_LOG";
pub const RUST_LOG_TELEMETRY: &str = "RUST_LOG_TELEMETRY";
const RUST_LOG_FORMAT: &str = "RUST_LOG_FORMAT";
/// Default size of log write channel, if the channel is full, logs will be dropped
pub const CHANNEL_SIZE: usize = 10000;
const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);
const FILTER_REFRESH_INTERVAL: Duration =
    Duration::from_secs(5 /* minutes */ * 60 /* seconds */);

/// Note: To disable length limits, set `RUST_LOG_FIELD_MAX_LEN` to -1.
const RUST_LOG_FIELD_MAX_LEN_ENV_VAR: &str = "RUST_LOG_FIELD_MAX_LEN";
static RUST_LOG_FIELD_MAX_LEN: Lazy<usize> = Lazy::new(|| {
    env::var(RUST_LOG_FIELD_MAX_LEN_ENV_VAR)
        .ok()
        .and_then(|value| i64::from_str(&value).map(|value| value as usize).ok())
        .unwrap_or(TruncatedLogString::DEFAULT_MAX_LEN)
});

struct TruncatedLogString(String);

impl TruncatedLogString {
    const DEFAULT_MAX_LEN: usize = 10 * 1024;
    const TRUNCATION_SUFFIX: &'static str = "(truncated)";

    fn new(s: String) -> Self {
        let mut truncated = s;

        if truncated.len() > RUST_LOG_FIELD_MAX_LEN.saturating_add(Self::TRUNCATION_SUFFIX.len()) {
            truncated.truncate(*RUST_LOG_FIELD_MAX_LEN);
            truncated.push_str(Self::TRUNCATION_SUFFIX);
        }
        TruncatedLogString(truncated)
    }
}

impl DerefMut for TruncatedLogString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for TruncatedLogString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for TruncatedLogString {
    fn from(value: String) -> Self {
        TruncatedLogString::new(value)
    }
}

impl From<TruncatedLogString> for String {
    fn from(val: TruncatedLogString) -> Self {
        val.0
    }
}

#[derive(EnumString)]
#[strum(serialize_all = "lowercase")]
enum LogFormat {
    Json,
    Text,
}

/// A single log entry emitted by a logging macro with associated metadata
#[derive(Debug)]
pub struct LogEntry {
    metadata: Metadata,
    thread_name: Option<String>,
    /// The program backtrace taken when the event occurred. Backtraces
    /// are only supported for errors and must be configured.
    backtrace: Option<String>,
    hostname: Option<&'static str>,
    namespace: Option<&'static str>,
    timestamp: String,
    data: BTreeMap<Key, serde_json::Value>,
    message: Option<String>,
    peer_id: Option<&'static str>,
    chain_id: Option<u8>,
    git_hash: Option<String>,
}

// implement custom serializer for LogEntry since we want to promote the `metadata.level` field into a top-level `level` field
// and prefix the remaining metadata attributes as `source.<metadata_field>` which can't be expressed with serde macros alone.
impl Serialize for LogEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("LogEntry", 9)?;
        state.serialize_field("level", &self.metadata.level())?;
        state.serialize_field("source", &self.metadata)?;
        if let Some(thread_name) = &self.thread_name {
            state.serialize_field("thread_name", thread_name)?;
        }
        if let Some(hostname) = &self.hostname {
            state.serialize_field("hostname", hostname)?;
        }
        if let Some(namespace) = &self.namespace {
            state.serialize_field("namespace", namespace)?;
        }
        state.serialize_field("timestamp", &self.timestamp)?;
        if let Some(message) = &self.message {
            state.serialize_field("message", message)?;
        }
        if !&self.data.is_empty() {
            state.serialize_field("data", &self.data)?;
        }
        if let Some(backtrace) = &self.backtrace {
            state.serialize_field("backtrace", backtrace)?;
        }
        if let Some(peer_id) = &self.peer_id {
            state.serialize_field("peer_id", peer_id)?;
        }
        if let Some(git_hash) = &self.git_hash {
            state.serialize_field("git_hash", git_hash)?;
        }
        state.end()
    }
}

impl LogEntry {
    fn new(event: &Event, thread_name: Option<&str>, enable_backtrace: bool) -> Self {
        use crate::{Value, Visitor};

        struct JsonVisitor<'a>(&'a mut BTreeMap<Key, serde_json::Value>);

        impl Visitor for JsonVisitor<'_> {
            fn visit_pair(&mut self, key: Key, value: Value<'_>) {
                let v = match value {
                    Value::Debug(d) => serde_json::Value::String(
                        TruncatedLogString::from(format!("{:?}", d)).into(),
                    ),
                    Value::Display(d) => {
                        serde_json::Value::String(TruncatedLogString::from(d.to_string()).into())
                    },
                    Value::Serde(s) => match serde_json::to_value(s) {
                        Ok(value) => value,
                        Err(e) => {
                            // Log and skip the value that can't be serialized
                            eprintln!("error serializing structured log: {} for key {:?}", e, key);
                            return;
                        },
                    },
                };

                self.0.insert(key, v);
            }
        }

        let metadata = *event.metadata();
        let thread_name = thread_name.map(ToOwned::to_owned);
        let message = event
            .message()
            .map(fmt::format)
            .map(|s| TruncatedLogString::from(s).into());

        static HOSTNAME: Lazy<Option<String>> = Lazy::new(|| {
            hostname::get()
                .ok()
                .and_then(|name| name.into_string().ok())
        });

        static NAMESPACE: Lazy<Option<String>> =
            Lazy::new(|| env::var("KUBERNETES_NAMESPACE").ok());

        let hostname = HOSTNAME.as_deref();
        let namespace = NAMESPACE.as_deref();

        let peer_id: Option<&str>;
        let chain_id: Option<u8>;
        let full_git_hash: Option<&str>;

        #[cfg(node_identity)]
        {
            peer_id = aptos_node_identity::peer_id_as_str();
            chain_id = aptos_node_identity::chain_id().map(|chain_id| chain_id.id());
            full_git_hash = aptos_node_identity::git_hash();
        }

        #[cfg(not(node_identity))]
        {
            peer_id = None;
            chain_id = None;
            full_git_hash = None;
        }

        let backtrace = if enable_backtrace && matches!(metadata.level(), Level::Error) {
            let mut backtrace = Backtrace::new();
            let mut frames = backtrace.frames().to_vec();
            if frames.len() > 3 {
                frames.drain(0..3); // Remove the first 3 unnecessary frames to simplify backtrace
            }
            backtrace = frames.into();
            Some(format!("{:?}", backtrace))
        } else {
            None
        };

        let mut data = BTreeMap::new();
        for schema in event.keys_and_values() {
            schema.visit(&mut JsonVisitor(&mut data));
        }

        let short_git_hash = full_git_hash.map(|full_git_hash| {
            if full_git_hash.len() >= 7 {
                full_git_hash[..7].to_string()
            } else {
                full_git_hash.to_string()
            }
        });

        Self {
            metadata,
            thread_name,
            backtrace,
            hostname,
            namespace,
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true),
            data,
            message,
            peer_id,
            chain_id,
            git_hash: short_git_hash,
        }
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn thread_name(&self) -> Option<&str> {
        self.thread_name.as_deref()
    }

    pub fn backtrace(&self) -> Option<&str> {
        self.backtrace.as_deref()
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname
    }

    pub fn namespace(&self) -> Option<&str> {
        self.namespace
    }

    pub fn timestamp(&self) -> &str {
        self.timestamp.as_str()
    }

    pub fn data(&self) -> &BTreeMap<Key, serde_json::Value> {
        &self.data
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn peer_id(&self) -> Option<&str> {
        self.peer_id
    }

    pub fn chain_id(&self) -> Option<u8> {
        self.chain_id
    }

    pub fn git_hash(&self) -> Option<&str> {
        self.git_hash.as_deref()
    }
}

/// A builder for a `AptosData`, configures what, where, and how to write logs.
pub struct AptosDataBuilder {
    channel_size: usize,
    tokio_console_port: Option<u16>,
    enable_backtrace: bool,
    level: Level,
    remote_level: Level,
    telemetry_level: Level,
    printer: Option<Box<dyn Writer>>,
    remote_log_tx: Option<channel::mpsc::Sender<TelemetryLog>>,
    is_async: bool,
    enable_telemetry_flush: bool,
    custom_format: Option<fn(&LogEntry) -> Result<String, fmt::Error>>,
}

impl AptosDataBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            channel_size: CHANNEL_SIZE,
            tokio_console_port: None,
            enable_backtrace: false,
            level: Level::Info,
            remote_level: Level::Info,
            telemetry_level: Level::Warn,
            printer: Some(Box::new(StdoutWriter::new())),
            remote_log_tx: None,
            is_async: false,
            enable_telemetry_flush: true,
            custom_format: None,
        }
    }

    pub fn enable_backtrace(&mut self) -> &mut Self {
        self.enable_backtrace = true;
        self
    }

    pub fn level(&mut self, level: Level) -> &mut Self {
        self.level = level;
        self
    }

    pub fn remote_level(&mut self, level: Level) -> &mut Self {
        self.remote_level = level;
        self
    }

    pub fn telemetry_level(&mut self, level: Level) -> &mut Self {
        self.telemetry_level = level;
        self
    }

    pub fn channel_size(&mut self, channel_size: usize) -> &mut Self {
        self.channel_size = channel_size;
        self
    }

    pub fn printer(&mut self, printer: Box<dyn Writer + Send + Sync + 'static>) -> &mut Self {
        self.printer = Some(printer);
        self
    }

    pub fn tokio_console_port(&mut self, tokio_console_port: Option<u16>) -> &mut Self {
        self.tokio_console_port = tokio_console_port;
        self
    }

    pub fn remote_log_tx(
        &mut self,
        remote_log_tx: channel::mpsc::Sender<TelemetryLog>,
    ) -> &mut Self {
        self.remote_log_tx = Some(remote_log_tx);
        self
    }

    pub fn is_async(&mut self, is_async: bool) -> &mut Self {
        self.is_async = is_async;
        self
    }

    pub fn enable_telemetry_flush(&mut self, enable_telemetry_flush: bool) -> &mut Self {
        self.enable_telemetry_flush = enable_telemetry_flush;
        self
    }

    pub fn custom_format(
        &mut self,
        format: fn(&LogEntry) -> Result<String, fmt::Error>,
    ) -> &mut Self {
        self.custom_format = Some(format);
        self
    }

    pub fn init(&mut self) {
        self.build();
    }

    fn build_filter(&self) -> FilterTuple {
        let local_filter = {
            let mut filter_builder = Filter::builder();

            if env::var(RUST_LOG).is_ok() {
                filter_builder.with_env(RUST_LOG);
            } else {
                filter_builder.filter_level(self.level.into());
            }

            filter_builder.build()
        };
        let telemetry_filter = {
            let mut filter_builder = Filter::builder();

            if self.is_async && self.remote_log_tx.is_some() {
                if env::var(RUST_LOG_TELEMETRY).is_ok() {
                    filter_builder.with_env(RUST_LOG_TELEMETRY);
                } else {
                    filter_builder.filter_level(self.telemetry_level.into());
                }
            } else {
                filter_builder.filter_level(LevelFilter::Off);
            }

            filter_builder.build()
        };

        FilterTuple {
            local_filter,
            telemetry_filter,
        }
    }

    fn build_logger(&mut self) -> Arc<AptosData> {
        let filter = self.build_filter();

        if let Ok(log_format) = env::var(RUST_LOG_FORMAT) {
            let log_format = LogFormat::from_str(&log_format).unwrap();
            self.custom_format = match log_format {
                LogFormat::Json => Some(json_format),
                LogFormat::Text => Some(text_format),
            }
        }

        if self.is_async {
            let (sender, receiver) = sync::mpsc::sync_channel(self.channel_size);
            let mut remote_tx = None;
            if let Some(tx) = &self.remote_log_tx {
                remote_tx = Some(tx.clone());
            }

            let logger = Arc::new(AptosData {
                enable_backtrace: self.enable_backtrace,
                sender: Some(sender),
                printer: None,
                filter: RwLock::new(filter),
                enable_telemetry_flush: self.enable_telemetry_flush,
                formatter: self.custom_format.take().unwrap_or(text_format),
            });
            let service = LoggerService {
                receiver,
                printer: self.printer.take(),
                facade: logger.clone(),
                remote_tx,
            };

            thread::spawn(move || service.run());
            logger
        } else {
            Arc::new(AptosData {
                enable_backtrace: self.enable_backtrace,
                sender: None,
                printer: self.printer.take(),
                filter: RwLock::new(filter),
                enable_telemetry_flush: self.enable_telemetry_flush,
                formatter: self.custom_format.take().unwrap_or(text_format),
            })
        }
    }

    pub fn build(&mut self) -> Arc<AptosData> {
        let logger = self.build_logger();

        let tokio_console_port = if cfg!(feature = "tokio-console") {
            self.tokio_console_port
        } else {
            None
        };

        crate::logger::set_global_logger(logger.clone(), tokio_console_port);
        logger
    }
}

/// A combination of `Filter`s to control where logs are written
pub struct FilterTuple {
    /// The local printer `Filter` to control what is logged in text output
    local_filter: Filter,
    /// The logging `Filter` to control what is sent to telemetry service
    telemetry_filter: Filter,
}

impl FilterTuple {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.local_filter.enabled(metadata) || self.telemetry_filter.enabled(metadata)
    }
}

pub struct AptosData {
    enable_backtrace: bool,
    sender: Option<sync::mpsc::SyncSender<LoggerServiceEvent>>,
    printer: Option<Box<dyn Writer>>,
    filter: RwLock<FilterTuple>,
    enable_telemetry_flush: bool,
    pub(crate) formatter: fn(&LogEntry) -> Result<String, fmt::Error>,
}

impl AptosData {
    pub fn builder() -> AptosDataBuilder {
        AptosDataBuilder::new()
    }

    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> AptosDataBuilder {
        Self::builder()
    }

    pub fn init_for_testing() {
        // Create the Aptos Data Builder
        let mut builder = Self::builder();
        builder
            .is_async(false)
            .enable_backtrace()
            .printer(Box::new(StdoutWriter::new()));

        // If RUST_LOG wasn't specified, default to Debug logging
        if env::var(RUST_LOG).is_err() {
            builder.level(Level::Debug);
        }

        builder.build();
    }

    pub fn set_filter(&self, filter_tuple: FilterTuple) {
        *self.filter.write() = filter_tuple;
    }

    pub fn set_local_filter(&self, filter: Filter) {
        self.filter.write().local_filter = filter;
    }

    pub fn set_telemetry_filter(&self, filter: Filter) {
        self.filter.write().telemetry_filter = filter;
    }

    fn send_entry(&self, entry: LogEntry) {
        if let Some(printer) = &self.printer {
            let s = (self.formatter)(&entry).expect("Unable to format");
            printer.write(s);
        }

        if let Some(sender) = &self.sender {
            if sender
                .try_send(LoggerServiceEvent::LogEntry(entry))
                .is_err()
            {
                STRUCT_LOG_QUEUE_ERROR_COUNT.inc();
            }
        }
    }
}

impl Logger for AptosData {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.read().enabled(metadata)
    }

    fn record(&self, event: &Event) {
        let entry = LogEntry::new(
            event,
            ::std::thread::current().name(),
            self.enable_backtrace,
        );

        self.send_entry(entry)
    }

    fn flush(&self) {
        if let Some(sender) = &self.sender {
            let (oneshot_sender, oneshot_receiver) = sync::mpsc::sync_channel(1);
            match sender.try_send(LoggerServiceEvent::Flush(oneshot_sender)) {
                Ok(_) => {
                    if let Err(err) = oneshot_receiver.recv_timeout(FLUSH_TIMEOUT) {
                        eprintln!("[Logging] Unable to flush recv: {}", err);
                    }
                },
                Err(err) => {
                    eprintln!("[Logging] Unable to flush send: {}", err);
                    std::thread::sleep(FLUSH_TIMEOUT);
                },
            }
        }
    }
}

enum LoggerServiceEvent {
    LogEntry(LogEntry),
    Flush(sync::mpsc::SyncSender<()>),
}

/// A service for running a log listener, that will continually export logs through a local printer
/// or to a `AptosData` for external logging.
struct LoggerService {
    receiver: sync::mpsc::Receiver<LoggerServiceEvent>,
    printer: Option<Box<dyn Writer>>,
    facade: Arc<AptosData>,
    remote_tx: Option<channel::mpsc::Sender<TelemetryLog>>,
}

impl LoggerService {
    pub fn run(mut self) {
        let mut telemetry_writer = self.remote_tx.take().map(TelemetryLogWriter::new);

        for event in &self.receiver {
            match event {
                LoggerServiceEvent::LogEntry(entry) => {
                    PROCESSED_STRUCT_LOG_COUNT.inc();
                    match entry.metadata.level() {
                        Level::Error => ERROR_LOG_COUNT.inc(),
                        Level::Warn => WARN_LOG_COUNT.inc(),
                        Level::Info => INFO_LOG_COUNT.inc(),
                        _ => {},
                    }

                    if let Some(printer) = &mut self.printer {
                        if self
                            .facade
                            .filter
                            .read()
                            .local_filter
                            .enabled(&entry.metadata)
                        {
                            let s = (self.facade.formatter)(&entry).expect("Unable to format");
                            printer.write_buferred(s);
                        }
                    }

                    if let Some(writer) = &mut telemetry_writer {
                        if self
                            .facade
                            .filter
                            .read()
                            .telemetry_filter
                            .enabled(&entry.metadata)
                        {
                            let s = json_format(&entry).expect("Unable to format");
                            let _ = writer.write(s);
                        }
                    }
                },
                LoggerServiceEvent::Flush(sender) => {
                    // Flush is only done on TelemetryLogWriter
                    if let Some(writer) = &mut telemetry_writer {
                        if self.facade.enable_telemetry_flush {
                            match writer.flush() {
                                Ok(rx) => {
                                    if let Err(err) = rx.recv_timeout(FLUSH_TIMEOUT) {
                                        sample!(
                                            SampleRate::Duration(Duration::from_secs(60)),
                                            eprintln!("Timed out flushing telemetry: {}", err)
                                        );
                                    }
                                },
                                Err(err) => {
                                    sample!(
                                        SampleRate::Duration(Duration::from_secs(60)),
                                        eprintln!("Failed to flush telemetry: {}", err)
                                    );
                                },
                            }
                        }
                    }
                    let _ = sender.send(());
                },
            }
        }
    }
}

/// A trait encapsulating the operations required for writing logs.
pub trait Writer: Send + Sync {
    /// Write the log.
    fn write(&self, log: String);

    /// Write the log in an async task.
    fn write_buferred(&mut self, log: String);
}

/// A struct for writing logs to stdout
struct StdoutWriter {
    buffer: std::io::BufWriter<Stdout>,
}

impl StdoutWriter {
    pub fn new() -> Self {
        let buffer = std::io::BufWriter::new(std::io::stdout());
        Self { buffer }
    }
}
impl Writer for StdoutWriter {
    /// Write log to stdout
    fn write(&self, log: String) {
        println!("{}", log);
    }

    fn write_buferred(&mut self, log: String) {
        self.buffer
            .write_fmt(format_args!("{}\n", log))
            .unwrap_or_default();
    }
}

/// A struct for writing logs to a file
pub struct FileWriter {
    log_file: RwLock<std::fs::File>,
}

impl FileWriter {
    pub fn new(log_file: std::path::PathBuf) -> Self {
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_file)
            .expect("Unable to open log file");
        Self {
            log_file: RwLock::new(file),
        }
    }
}

impl Writer for FileWriter {
    /// Write to file
    fn write(&self, log: String) {
        if let Err(err) = writeln!(self.log_file.write(), "{}", log) {
            eprintln!("Unable to write to log file: {}", err);
        }
    }

    fn write_buferred(&mut self, log: String) {
        self.write(log);
    }
}

/// Converts a record into a string representation:
/// UNIX_TIMESTAMP LOG_LEVEL [thread_name] FILE:LINE MESSAGE JSON_DATA
/// Example:
/// 2020-03-07 05:03:03 INFO [thread_name] common/aptos-logger/src/lib.rs:261 Hello { "world": true }
fn text_format(entry: &LogEntry) -> Result<String, fmt::Error> {
    use std::fmt::Write;

    let mut w = String::new();
    write!(w, "{}", entry.timestamp)?;

    if let Some(thread_name) = &entry.thread_name {
        write!(w, " [{}]", thread_name)?;
    }

    write!(
        w,
        " {} {}",
        entry.metadata.level(),
        entry.metadata.source_path()
    )?;

    if let Some(message) = &entry.message {
        write!(w, " {}", message)?;
    }

    if !entry.data.is_empty() {
        write!(w, " {}", serde_json::to_string(&entry.data).unwrap())?;
    }

    Ok(w)
}

// converts a record into json format
fn json_format(entry: &LogEntry) -> Result<String, fmt::Error> {
    match serde_json::to_string(&entry) {
        Ok(s) => Ok(s),
        Err(_) => {
            // TODO: Improve the error handling here. Currently we're just increasing some misleadingly-named metric and dropping any context on why this could not be deserialized.
            STRUCT_LOG_PARSE_ERROR_COUNT.inc();
            Err(fmt::Error)
        },
    }
}

/// Periodically rebuilds the filter and replaces the current logger filter.
/// This is useful for dynamically changing log levels at runtime via existing
/// environment variables such as `RUST_LOG_TELEMETRY`.
pub struct LoggerFilterUpdater {
    logger: Arc<AptosData>,
    logger_builder: AptosDataBuilder,
}

impl LoggerFilterUpdater {
    pub fn new(logger: Arc<AptosData>, logger_builder: AptosDataBuilder) -> Self {
        Self {
            logger,
            logger_builder,
        }
    }

    pub async fn run(self) {
        let mut interval = time::interval(FILTER_REFRESH_INTERVAL);
        loop {
            interval.tick().await;

            self.update_filter();
        }
    }

    fn update_filter(&self) {
        // TODO: check for change to env var before rebuilding filter.
        let filter = self.logger_builder.build_filter();
        self.logger.set_filter(filter);
    }
}

#[cfg(test)]
mod tests {
    use super::{text_format, AptosData, LogEntry};
    use crate::{
        aptos_logger::{json_format, TruncatedLogString, RUST_LOG_TELEMETRY},
        debug, error, info,
        logger::Logger,
        telemetry_log_writer::TelemetryLog,
        trace, warn, AptosDataBuilder, Event, Key, KeyValue, Level, LoggerFilterUpdater, Metadata,
        Schema, Value, Visitor, Writer,
    };
    use chrono::{DateTime, Utc};
    use futures::StreamExt;
    #[cfg(test)]
    use pretty_assertions::assert_eq;
    use serde_json::Value as JsonValue;
    use std::{
        env,
        sync::{
            mpsc::{self, Receiver, SyncSender},
            Arc, Mutex,
        },
        thread,
        time::Duration,
    };
    use tokio::time;

    pub struct MockWriter {
        pub logs: Arc<Mutex<Vec<String>>>,
    }

    impl Writer for MockWriter {
        fn write(&self, log: String) {
            let mut logs = self.logs.lock().unwrap();
            logs.push(log);
        }

        fn write_buferred(&mut self, log: String) {
            self.write(log);
        }
    }

    impl MockWriter {
        pub fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
            let logs = Arc::new(Mutex::new(Vec::new()));
            (Self { logs: logs.clone() }, logs)
        }
    }

    #[derive(serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Enum {
        FooBar,
    }

    struct TestSchema<'a> {
        foo: usize,
        bar: &'a Enum,
    }

    impl Schema for TestSchema<'_> {
        fn visit(&self, visitor: &mut dyn Visitor) {
            visitor.visit_pair(Key::new("foo"), Value::from_serde(&self.foo));
            visitor.visit_pair(Key::new("bar"), Value::from_serde(&self.bar));
        }
    }

    struct LogStream {
        sender: SyncSender<LogEntry>,
        enable_backtrace: bool,
    }

    impl LogStream {
        fn new(enable_backtrace: bool) -> (Self, Receiver<LogEntry>) {
            let (sender, receiver) = mpsc::sync_channel(1024);
            let log_stream = Self {
                sender,
                enable_backtrace,
            };
            (log_stream, receiver)
        }
    }

    impl Logger for LogStream {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Debug
        }

        fn record(&self, event: &Event) {
            let entry = LogEntry::new(
                event,
                ::std::thread::current().name(),
                self.enable_backtrace,
            );
            self.sender.send(entry).unwrap();
        }

        fn flush(&self) {}
    }

    fn set_test_logger() -> Receiver<LogEntry> {
        let (logger, receiver) = LogStream::new(true);
        let logger = Arc::new(logger);
        crate::logger::set_global_logger(logger, None);
        receiver
    }

    // TODO: Find a better mechanism for testing that allows setting the logger not globally
    #[test]
    fn basic() {
        let receiver = set_test_logger();
        let number = 12345;

        // Send an info log
        let before = Utc::now();
        let mut line_num = line!();
        info!(
            TestSchema {
                foo: 5,
                bar: &Enum::FooBar
            },
            test = true,
            category = "name",
            KeyValue::new("display", Value::from_display(&number)),
            "This is a log"
        );

        let after = Utc::now();

        let mut entry = receiver.recv().unwrap();

        // Ensure standard fields are filled
        assert_eq!(entry.metadata.level(), Level::Info);
        assert_eq!(
            entry.metadata.target(),
            module_path!().split("::").next().unwrap()
        );
        assert_eq!(entry.message.as_deref(), Some("This is a log"));
        assert!(entry.backtrace.is_none());

        // Ensure json formatter works
        // hardcoding a timestamp and hostname to make the tests deterministic and not depend on environment
        let original_timestamp = entry.timestamp;
        entry.timestamp = String::from("2022-07-24T23:42:29.540278Z");
        entry.hostname = Some("test-host");
        entry.git_hash = Some("1234567".to_string());
        line_num += 1;
        let thread_name = thread::current().name().map(|s| s.to_string()).unwrap();

        let expected = format!("{{\"level\":\"INFO\",\"source\":{{\"package\":\"aptos_logger\",\"file\":\"crates/aptos-logger/src/aptos_logger.rs:{line_num}\"}},\"thread_name\":\"{thread_name}\",\"hostname\":\"test-host\",\"timestamp\":\"2022-07-24T23:42:29.540278Z\",\"message\":\"This is a log\",\"data\":{{\"bar\":\"foo_bar\",\"category\":\"name\",\"display\":\"12345\",\"foo\":5,\"test\":true}},\"git_hash\":\"1234567\"}}");

        assert_eq!(json_format(&entry).unwrap(), expected);

        entry.timestamp = original_timestamp;

        // Log time should be the time the structured log entry was created
        let timestamp = DateTime::parse_from_rfc3339(&entry.timestamp).unwrap();
        let timestamp: DateTime<Utc> = DateTime::from(timestamp);
        assert!(before <= timestamp && timestamp <= after);

        // Ensure data stored is the right type
        assert_eq!(
            entry.data.get(&Key::new("foo")).and_then(JsonValue::as_u64),
            Some(5)
        );
        assert_eq!(
            entry.data.get(&Key::new("bar")).and_then(JsonValue::as_str),
            Some("foo_bar")
        );
        assert_eq!(
            entry
                .data
                .get(&Key::new("display"))
                .and_then(JsonValue::as_str),
            Some(format!("{}", number)).as_deref(),
        );
        assert_eq!(
            entry
                .data
                .get(&Key::new("test"))
                .and_then(JsonValue::as_bool),
            Some(true),
        );
        assert_eq!(
            entry
                .data
                .get(&Key::new("category"))
                .and_then(JsonValue::as_str),
            Some("name"),
        );

        // Test error logs contain backtraces
        error!("This is an error log");
        let entry = receiver.recv().unwrap();
        assert!(entry.backtrace.is_some());

        // Test all log levels work properly
        // Tracing should be skipped because the Logger was setup to skip Tracing events
        trace!("trace");
        debug!("debug");
        info!("info");
        warn!("warn");
        error!("error");

        let levels = &[Level::Debug, Level::Info, Level::Warn, Level::Error];

        for level in levels {
            let entry = receiver.recv().unwrap();
            assert_eq!(entry.metadata.level(), *level);
        }

        // Verify that the thread name is properly included
        let handler = thread::Builder::new()
            .name("named thread".into())
            .spawn(|| info!("thread"))
            .unwrap();

        handler.join().unwrap();
        let entry = receiver.recv().unwrap();
        assert_eq!(entry.thread_name.as_deref(), Some("named thread"));

        // Test Debug and Display inputs
        let debug_struct = DebugStruct {};
        let display_struct = DisplayStruct {};

        error!(identifier = ?debug_struct, "Debug test");
        error!(identifier = ?debug_struct, other = "value", "Debug2 test");
        error!(identifier = %display_struct, "Display test");
        error!(identifier = %display_struct, other = "value", "Display2 test");
        error!("Literal" = ?debug_struct, "Debug test");
        error!("Literal" = ?debug_struct, other = "value", "Debug test");
        error!("Literal" = %display_struct, "Display test");
        error!("Literal" = %display_struct, other = "value", "Display2 test");
        error!("Literal" = %display_struct, other = "value", identifier = ?debug_struct, "Mixed test");
    }

    struct DebugStruct {}

    impl std::fmt::Debug for DebugStruct {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "DebugStruct!")
        }
    }

    struct DisplayStruct {}

    impl std::fmt::Display for DisplayStruct {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "DisplayStruct!")
        }
    }

    fn new_async_logger() -> (AptosDataBuilder, Arc<AptosData>) {
        let mut logger_builder = AptosDataBuilder::new();
        let (remote_log_tx, _) = futures::channel::mpsc::channel(10);
        let logger = logger_builder
            .remote_log_tx(remote_log_tx)
            .is_async(true)
            .build_logger();
        (logger_builder, logger)
    }

    fn new_text_logger() -> (
        Arc<AptosData>,
        Arc<Mutex<Vec<String>>>,
        futures::channel::mpsc::Receiver<TelemetryLog>,
    ) {
        let (tx, rx) = futures::channel::mpsc::channel(100);
        let (mock_writer, logs) = MockWriter::new();

        let logger = AptosDataBuilder::new()
            .level(Level::Debug)
            .telemetry_level(Level::Debug)
            .remote_log_tx(tx)
            .custom_format(text_format)
            .printer(Box::new(mock_writer))
            .is_async(true)
            .build_logger();

        (logger, logs, rx)
    }

    #[tokio::test]
    async fn telemetry_logs_always_json_formatted() {
        let (logger, local_logs, mut rx) = new_text_logger();

        let metadata = Metadata::new(
            Level::Info,
            env!("CARGO_CRATE_NAME"),
            module_path!(),
            concat!(file!(), ':', line!()),
        );

        let event = &Event::new(&metadata, Some(format_args!("This is a log message")), &[]);
        logger.record(event);
        logger.flush();

        {
            let local_logs = local_logs.lock().unwrap();
            assert!(serde_json::from_str::<JsonValue>(&local_logs[0]).is_err());
        }

        let timeout_duration = Duration::from_secs(5);
        if let Ok(Some(TelemetryLog::Log(telemetry_log))) =
            time::timeout(timeout_duration, rx.next()).await
        {
            assert!(
                serde_json::from_str::<JsonValue>(&telemetry_log).is_ok(),
                "Telemetry logs not in JSON format: {}",
                telemetry_log
            );
        } else {
            panic!("Timed out waiting for telemetry log");
        }
    }

    #[test]
    fn test_logger_filter_updater() {
        let (logger_builder, logger) = new_async_logger();
        let debug_metadata = &Metadata::new(Level::Debug, "target", "module_path", "source_path");

        assert!(!logger
            .filter
            .read()
            .telemetry_filter
            .enabled(debug_metadata));

        std::env::set_var(RUST_LOG_TELEMETRY, "debug");

        let updater = LoggerFilterUpdater::new(logger.clone(), logger_builder);
        updater.update_filter();

        assert!(logger
            .filter
            .read()
            .telemetry_filter
            .enabled(debug_metadata));

        std::env::set_var(RUST_LOG_TELEMETRY, "debug;hyper=off"); // log values should be separated by commas not semicolons.
        updater.update_filter();

        assert!(!logger
            .filter
            .read()
            .telemetry_filter
            .enabled(debug_metadata));

        std::env::set_var(RUST_LOG_TELEMETRY, "debug,hyper=off"); // log values should be separated by commas not semicolons.
        updater.update_filter();

        assert!(logger
            .filter
            .read()
            .telemetry_filter
            .enabled(debug_metadata));

        assert!(!logger
            .filter
            .read()
            .telemetry_filter
            .enabled(&Metadata::new(
                Level::Error,
                "target",
                "hyper",
                "source_path"
            )));
    }

    #[test]
    fn test_log_event_truncation() {
        let log_entry = LogEntry::new(
            &Event::new(
                &Metadata::new(Level::Error, "target", "hyper", "source_path"),
                Some(format_args!(
                    "{}",
                    "a".repeat(2 * TruncatedLogString::DEFAULT_MAX_LEN)
                )),
                &[
                    &KeyValue::new(
                        "key1",
                        Value::Debug(&"x".repeat(2 * TruncatedLogString::DEFAULT_MAX_LEN)),
                    ),
                    &KeyValue::new(
                        "key2",
                        Value::Display(&"y".repeat(2 * TruncatedLogString::DEFAULT_MAX_LEN)),
                    ),
                ],
            ),
            Some("test_thread"),
            false,
        );
        assert_eq!(
            log_entry.message,
            Some(format!(
                "{}{}",
                "a".repeat(TruncatedLogString::DEFAULT_MAX_LEN),
                "(truncated)"
            ))
        );
        assert_eq!(
            log_entry.data().first_key_value().unwrap().1,
            &serde_json::Value::String(format!(
                "\"{}{}",
                "x".repeat(TruncatedLogString::DEFAULT_MAX_LEN - 1),
                "(truncated)"
            ))
        );
        assert_eq!(
            log_entry.data().last_key_value().unwrap().1,
            &serde_json::Value::String(format!(
                "{}{}",
                "y".repeat(TruncatedLogString::DEFAULT_MAX_LEN),
                "(truncated)"
            ))
        );
    }
}
