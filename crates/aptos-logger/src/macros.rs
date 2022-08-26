// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Macros for sending logs at predetermined log `Level`s
#[cfg(not(feature = "aptos-console"))]
#[macro_export]
macro_rules! spawn_named {
      ($name:expr, $func:expr) => { tokio::spawn($func); };
      ($name:expr, $handler:expr, $func:expr) => { $handler.spawn($func); };
      ($name:expr, $async:ident = async; $clojure:block) => { tokio::spawn( async $clojure); };
      ($name:expr, $handler:expr, $async:ident = async; $clojure:block) => { $handler.spawn( async $clojure); };
      ($name:expr, $async:ident = async ; $move:ident = move; $clojure:block) => { tokio::spawn( async move $clojure); };
      ($name:expr, $handler:expr, $async:ident = async ; $move:ident = move; $clojure:block) => { $handler.spawn( async move $clojure); };
  }

#[cfg(feature = "aptos-console")]
#[macro_export]
macro_rules! spawn_named {
      ($name:expr, $func:expr) => { tokio::task::Builder::new()
                                          .name($name)
                                          .spawn($func); };
      ($name:expr, $handle:expr, $func:expr) => { tokio::task::Builder::new()
                                                      .name($name)
                                                      .spawn_on($func, $handle); };

      ($name:expr, $async:ident = async; $clojure:block) => { tokio::task::Builder::new()
                                                                      .name($name)
                                                                      .spawn(async $clojure); };

      ($name:expr, $async:ident = async; $move:ident = move; $clojure:block) => { tokio::task::Builder::new()
                                                                      .name($name)
                                                                      .spawn(async move $clojure); };

      ($name:expr, $handler:expr, $async:ident = async; $clojure:block) => { tokio::task::Builder::new()
                                                                              .name($name)
                                                                              .spawn_on(async $clojure, $handler); };

      ($name:expr, $handler:expr, $async:ident = async; $move:ident = move; $clojure:block) => { tokio::task::Builder::new()
                                                                                                  .name($name)
                                                                                                  .spawn_on(async move $clojure, $handler); };

}

/// Log at the given level, it's recommended to use a specific level macro instead
#[macro_export]
macro_rules! log {
    // Entry, Log Level + stuff
    ($level:expr, $($args:tt)+) => {{
        const METADATA: $crate::Metadata = $crate::Metadata::new(
            $level,
            env!("CARGO_CRATE_NAME"),
            module_path!(),
            concat!(file!(), ':', line!()),
        );

        if METADATA.enabled() {
            $crate::Event::dispatch(
                &METADATA,
                $crate::fmt_args!($($args)+),
                $crate::schema!($($args)+),
            );
        }
    }};
}

#[macro_export]
macro_rules! enabled {
    ($level:expr) => {{
        const METADATA: $crate::Metadata = $crate::Metadata::new(
            $level,
            env!("CARGO_CRATE_NAME"),
            module_path!(),
            concat!(file!(), ':', line!()),
        );
        METADATA.enabled()
    }};
}

/// Log at the `trace` level
#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => {
        $crate::log!($crate::Level::Trace, $($arg)+)
    };
}

/// Log at the `debug` level
#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        $crate::log!($crate::Level::Debug, $($arg)+)
    };
}

/// Log at the `info` level
#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        $crate::log!($crate::Level::Info, $($arg)+)
    };
}

/// Log at the `warn` level
#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        $crate::log!($crate::Level::Warn, $($arg)+)
    };
}

/// Log at the `error` level
#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        $crate::log!($crate::Level::Error, $($arg)+)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! schema {
    //
    // base case
    //
    (@ { $(,)* $($val:expr),* $(,)* } $(,)*) => {
        &[ $($val),* ]
    };

    //
    // recursive cases
    //

    // format args
    (@ { $(,)* $($out:expr),* }, $template:literal, $($args:tt)*) => {
        $crate::schema!(
            @ { $($out),* }
        )
    };
    (@ { $(,)* $($out:expr),* }, $template:literal) => {
        $crate::schema!(
            @ { $($out),* }
        )
    };

    // Identifier Keys
    (@ { $(,)* $($out:expr),* }, $($k:ident).+ = $val:expr, $($args:tt)*) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($crate::__log_stringify!($($k).+), $crate::Value::from_serde(&$val)) },
            $($args)*
        )
    };

    (@ { $(,)* $($out:expr),* }, $($k:ident).+ = $val:expr) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($crate::__log_stringify!($($k).+), $crate::Value::from_serde(&$val)) },
        )
    };

    // Identifier Keys debug
    (@ { $(,)* $($out:expr),* }, $($k:ident).+ = ?$val:expr, $($args:tt)*) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($crate::__log_stringify!($($k).+), $crate::Value::from_debug(&$val)) },
            $($args)*
        )
    };

    (@ { $(,)* $($out:expr),* }, $($k:ident).+ = ?$val:expr) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($crate::__log_stringify!($($k).+), $crate::Value::from_debug($val)) },
        )
    };

    // Identifier Keys display
    (@ { $(,)* $($out:expr),* }, $($k:ident).+ = %$val:expr, $($args:tt)*) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($crate::__log_stringify!($($k).+), $crate::Value::from_display(&$val)) },
            $($args)*
        )
    };

    (@ { $(,)* $($out:expr),* }, $($k:ident).+ = %$val:expr) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($crate::__log_stringify!($($k).+), $crate::Value::from_display(&$val)) },
        )
    };

    // Literal Keys
    (@ { $(,)* $($out:expr),* }, $k:literal = $val:expr, $($args:tt)*) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($k, $crate::Value::from_serde(&$val)) },
            $($args)*
        )
    };

    (@ { $(,)* $($out:expr),* }, $k:literal = $val:expr) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($k, $crate::Value::from_serde(&$val)) },
        )
    };

    // Literal Keys debug
    (@ { $(,)* $($out:expr),* }, $k:literal = ?$val:expr, $($args:tt)*) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($k, $crate::Value::from_debug(&$val)) },
            $($args)*
        )
    };

    (@ { $(,)* $($out:expr),* }, $k:literal = ?$val:expr) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($k, $crate::Value::from_debug(&$val)) },
        )
    };

    // Literal Keys display
    (@ { $(,)* $($out:expr),* }, $k:literal = %$val:expr, $($args:tt)*) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($k, $crate::Value::from_display(&$val)) },
            $($args)*
        )
    };

    (@ { $(,)* $($out:expr),* }, $k:literal = %$val:expr) => {
        $crate::schema!(
            @ { $($out),*, &$crate::KeyValue::new($k, $crate::Value::from_display(&$val)) },
        )
    };

    // Lone Schemas
    (@ { $(,)* $($out:expr),* }, $schema:expr, $($args:tt)*) => {
        $crate::schema!(
            @ { $($out),*, &$schema },
            $($args)*
        )
    };
    (@ { $(,)* $($out:expr),* }, $schema:expr) => {
        $crate::schema!(
            @ { $($out),*, &$schema },
        )
    };

    //
    // entry
    //
    ($($args:tt)*) => {
        $crate::schema!(@ { }, $($args)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! fmt_args {
    //
    // base case
    //
    () => {
        None
    };

    // format args
    ($template:literal, $($args:tt)*) => {
        Some(::std::format_args!($template, $($args)*))
    };
    ($template:literal) => {
        Some(::std::format_args!($template))
    };

    // Identifier Keys
    ($($k:ident).+ = $val:expr, $($args:tt)*) => {
        $crate::fmt_args!(
            $($args)*
        )
    };
    ($($k:ident).+ = $val:expr) => {
        $crate::fmt_args!()
    };
    // Identifier Keys with Debug
    ($($k:ident).+ = ?$val:expr, $($args:tt)*) => {
        $crate::fmt_args!(
            $($args)*
        )
    };
    ($($k:ident).+ = ?$val:expr) => {
        $crate::fmt_args!()
    };
    // Identifier Keys with Display
    ($($k:ident).+ = %$val:expr, $($args:tt)*) => {
        $crate::fmt_args!(
            $($args)*
        )
    };
    ($($k:ident).+ = %$val:expr) => {
        $crate::fmt_args!()
    };

    // Literal Keys
    ($k:literal = $val:expr, $($args:tt)*) => {
        $crate::fmt_args!(
            $($args)*
        )
    };
    ($k:literal = $val:expr) => {
        $crate::fmt_args!()
    };
    // Literal Keys with Debug
    ($k:literal = ?$val:expr, $($args:tt)*) => {
        $crate::fmt_args!(
            $($args)*
        )
    };
    ($k:literal = ?$val:expr) => {
        $crate::fmt_args!()
    };
    // Literal Keys with Display
    ($k:literal = %$val:expr, $($args:tt)*) => {
        $crate::fmt_args!(
            $($args)*
        )
    };
    ($k:literal = %$val:expr) => {
        $crate::fmt_args!()
    };

    // Lone Schemas
    ($schema:expr, $($args:tt)*) => {
        $crate::fmt_args!(
            $($args)*
        )
    };
    ($schema:expr) => {
        $crate::fmt_args!()
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_stringify {
    ($s:expr) => {
        stringify!($s)
    };
}
