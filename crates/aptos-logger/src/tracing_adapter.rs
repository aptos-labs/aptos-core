// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{self as dl};
use std::{collections::BTreeMap, fmt};
use tracing::{
    field::Field,
    span::{Attributes, Id},
    Event, Level, Metadata,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// A layer that translates tracing events into aptos-logger events.
pub struct TracingToAptosDataLayer;

fn translate_level(level: &Level) -> Option<dl::Level> {
    if *level == Level::ERROR {
        return Some(dl::Level::Error);
    }
    if *level == Level::INFO {
        return Some(dl::Level::Info);
    }
    if *level == Level::DEBUG {
        return Some(dl::Level::Debug);
    }
    if *level == Level::TRACE {
        return Some(dl::Level::Trace);
    }
    if *level == Level::WARN {
        return Some(dl::Level::Warn);
    }
    None
}

struct SpanData {
    data: BTreeMap<String, String>,
    prefix: String,
}

impl SpanData {
    fn new(attrs: &Attributes<'_>, name: String) -> Self {
        let mut span = Self {
            data: BTreeMap::new(),
            prefix: name,
        };
        attrs.record(&mut span);
        span
    }
}

impl tracing::field::Visit for SpanData {
    fn record_str(&mut self, field: &Field, value: &str) {
        let name = format!("{}.{}", self.prefix, &field.name());
        self.data.insert(name, value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let name = format!("{}.{}", self.prefix, &field.name());
        self.data.insert(name, format!("{:?}", value));
    }
}

struct KeyValueVisitorAdapter<'a> {
    visitor: &'a mut dyn dl::Visitor,
}

impl tracing::field::Visit for KeyValueVisitorAdapter<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.visitor
            .visit_pair(dl::Key::new(field.name()), dl::Value::Display(&value))
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.visitor
            .visit_pair(dl::Key::new(field.name()), dl::Value::Debug(value))
    }
}

fn translate_metadata(metadata: &Metadata<'static>) -> Option<dl::Metadata> {
    let level = translate_level(metadata.level())?;

    Some(dl::Metadata::new(
        level,
        metadata.target(),
        metadata.module_path().unwrap_or(""),
        metadata.file().unwrap_or(""),
    ))
}

struct SpanValues {
    pairs: BTreeMap<String, String>,
}

impl dl::Schema for SpanValues {
    fn visit(&self, visitor: &mut dyn dl::Visitor) {
        for (key, value) in &self.pairs {
            visitor.visit_pair(
                dl::Key::new_owned(key.to_string()),
                dl::Value::from_display(&value),
            )
        }
    }
}

struct EventKeyValueAdapter<'a, 'b> {
    event: &'a Event<'b>,
}

impl dl::Schema for EventKeyValueAdapter<'_, '_> {
    fn visit(&self, visitor: &mut dyn dl::Visitor) {
        self.event.record(&mut KeyValueVisitorAdapter { visitor })
    }
}

impl<S> Layer<S> for TracingToAptosDataLayer
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Unable to load span; this is a bug");

        let prefix = {
            if let Some(parent) = span.parent() {
                // first, load the parent's span's, if present, to avoid
                // clobbering key/value pairs in the output.
                let parent_ext = parent.extensions();

                let data = parent_ext
                    .get::<SpanData>()
                    .expect("Parent does not have scuba data; this is a bug");

                // an unfortunate clone.
                Some(data.prefix.clone())
            } else {
                None
            }
        };

        let prefix = match prefix {
            Some(prefix) => format!("{}.{}", prefix, attrs.metadata().name()),
            None => attrs.metadata().name().to_string(),
        };
        let data = SpanData::new(attrs, prefix);
        span.extensions_mut().insert(data);
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        let metadata = match translate_metadata(event.metadata()) {
            Some(metadata) => metadata,
            None => {
                dl::warn!(
                    "[tracing-to-aptos-logger] failed to translate event due to unknown level {:?}",
                    event.metadata().level()
                );
                return;
            },
        };

        let mut acc = BTreeMap::new();
        if let Some(scope) = ctx.event_scope(event) {
            for data in scope {
                let ext = data.extensions();
                let data = ext
                    .get::<SpanData>()
                    .expect("span does not have data; this is a bug");

                acc.extend(data.data.clone())
            }
        }
        let values = acc;
        let data = SpanValues { pairs: values };

        // `tracing::Event` contains an implicit field named "message".
        // However I couldn't figure out a way to convert it to `fmt::Arguments` due to lifetime issues.
        // Therefore I'm omitting message argument to `Event::dispatch`.
        // This should generally be fine since the message will be translated as a normal record.
        if dl::logger::enabled(&metadata) {
            dl::Event::dispatch(&metadata, None, &[&EventKeyValueAdapter { event }, &data]);
        }
    }
}
