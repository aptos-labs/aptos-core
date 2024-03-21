use crate::config::options::IgnoreList;

/// Trait for types that can be used in `Config`.
pub trait ConfigType: Sized {
    /// Returns hint text for use in `Config::print_docs()`. For enum types, this is a
    /// pipe-separated list of variants; for other types it returns `<type>`.
    fn doc_hint() -> String;

    /// Return `true` if the variant (i.e. value of this type) is stable.
    ///
    /// By default, return true for all values. Enums annotated with `#[config_type]`
    /// are automatically implemented, based on the `#[unstable_variant]` annotation.
    fn stable_variant(&self) -> bool {
        true
    }
}

impl ConfigType for bool {
    fn doc_hint() -> String {
        String::from("<boolean>")
    }
}

impl ConfigType for usize {
    fn doc_hint() -> String {
        String::from("<unsigned integer>")
    }
}

impl ConfigType for isize {
    fn doc_hint() -> String {
        String::from("<signed integer>")
    }
}

impl ConfigType for String {
    fn doc_hint() -> String {
        String::from("<string>")
    }
}

impl ConfigType for IgnoreList {
    fn doc_hint() -> String {
        String::from("[<string>,..]")
    }
}

macro_rules! create_config {
    // Options passed in to the macro.
    //
    // - $i: the ident name of the option
    // - $ty: the type of the option value
    // - $def: the default value of the option
    // - $stb: true if the option is stable
    // - $dstring: description of the option
    ($($i:ident: $ty:ty, $def:expr, $stb:expr, $( $dstring:expr ),+ );+ $(;)*) => (
        #[cfg(test)]
        use std::collections::HashSet;

        use serde::{Deserialize, Serialize};

        #[derive(Clone)]
        #[allow(unreachable_pub)]
        pub struct Config {
            // For each config item, we store:
            //
            // - 0: true if the value has been access
            // - 1: true if the option was manually initialized
            // - 2: the option value
            // - 3: true if the option is unstable
            $($i: (Cell<bool>, bool, $ty, bool)),+
        }

        // Just like the Config struct but with each property wrapped
        // as Option<T>. This is used to parse a movefmt.toml that doesn't
        // specify all properties of `Config`.
        // We first parse into `PartialConfig`, then create a default `Config`
        // and overwrite the properties with corresponding values from `PartialConfig`.
        #[derive(Deserialize, Serialize, Clone)]
        #[allow(unreachable_pub)]
        pub struct PartialConfig {
            $(pub $i: Option<$ty>),+
        }

        // Macro hygiene won't allow us to make `set_$i()` methods on Config
        // for each item, so this struct is used to give the API to set values:
        // `config.set().option(false)`. It's pretty ugly. Consider replacing
        // with `config.set_option(false)` if we ever get a stable/usable
        // `concat_idents!()`.
        #[allow(unreachable_pub)]
        pub struct ConfigSetter<'a>(&'a mut Config);

        impl<'a> ConfigSetter<'a> {
            $(
            #[allow(unreachable_pub)]
            pub fn $i(&mut self, value: $ty) {
                (self.0).$i.2 = value;
                match stringify!($i) {
                    &_ => (),
                }
            }
            )+
        }

        // Query each option, returns true if the user set the option, false if
        // a default was used.
        #[allow(unreachable_pub)]
        pub struct ConfigWasSet<'a>(&'a Config);

        impl<'a> ConfigWasSet<'a> {
            $(
            #[allow(unreachable_pub)]
            pub fn $i(&self) -> bool {
                (self.0).$i.1
            }
            )+
        }

        impl Config {
            $(
            #[allow(unreachable_pub)]
            pub fn $i(&self) -> $ty {
                self.$i.0.set(true);
                self.$i.2.clone()
            }
            )+

            #[allow(unreachable_pub)]
            pub fn set(&mut self) -> ConfigSetter<'_> {
                ConfigSetter(self)
            }

            #[allow(unreachable_pub)]
            pub fn was_set(&self) -> ConfigWasSet<'_> {
                ConfigWasSet(self)
            }

            fn fill_from_parsed_config(mut self, parsed: PartialConfig) -> Config {
            $(
                if let Some(option_value) = parsed.$i {
                    let option_stable = self.$i.3;
                    if $crate::config::config_type::is_stable_option_and_value(
                        stringify!($i), option_stable, &option_value
                    ) {
                        self.$i.1 = true;
                        self.$i.2 = option_value;
                    }
                }
            )+
                self
            }

            /// Returns a hash set initialized with every user-facing config option name.
            #[cfg(test)]
            pub fn hash_set() -> HashSet<String> {
                let mut hash_set = HashSet::new();
                $(
                    hash_set.insert(stringify!($i).to_owned());
                )+
                hash_set
            }

            pub fn is_valid_name(name: &str) -> bool {
                match name {
                    $(
                        stringify!($i) => true,
                    )+
                        _ => false,
                }
            }

            #[allow(unreachable_pub)]
            pub fn is_valid_key_val(key: &str, val: &str) -> bool {
                match key {
                    $(
                        stringify!($i) => val.parse::<$ty>().is_ok(),
                    )+
                        _ => false,
                }
            }

            #[allow(unreachable_pub)]
            pub fn used_options(&self) -> PartialConfig {
                PartialConfig {
                    $(
                        $i: if self.$i.0.get() {
                                Some(self.$i.2.clone())
                            } else {
                                None
                            },
                    )+
                }
            }

            #[allow(unreachable_pub)]
            pub fn all_options(&self) -> PartialConfig {
                PartialConfig {
                    $(
                        $i: Some(self.$i.2.clone()),
                    )+
                }
            }

            #[allow(unreachable_pub)]
            pub fn override_value(&mut self, key: &str, val: &str)
            {
                match key {
                    $(
                        stringify!($i) => {
                            let option_value = val.parse::<$ty>()
                                .expect(&format!("Failed to parse override for {} (\"{}\") as a {}",
                                                 stringify!($i),
                                                 val,
                                                 stringify!($ty)));

                            // Users are currently allowed to set unstable
                            // options/variants via the `--config` options override.
                            // For now, do not validate whether the option or value is stable,
                            // just always set it.
                            self.$i.1 = true;
                            self.$i.2 = option_value;
                        }
                    )+
                    _ => panic!("Unknown config key in override: {}", key)
                }

                match key {
                    &_ => (),
                }
            }

            #[allow(unreachable_pub)]
            /// Returns `true` if the config key was explicitly set and is the default value.
            pub fn is_default(&self, key: &str) -> bool {
                $(
                    if let stringify!($i) = key {
                        return self.$i.1 && self.$i.2 == $def;
                    }
                 )+
                false
            }
        }

        // Template for the default configuration
        impl Default for Config {
            fn default() -> Config {
                Config {
                    $(
                        $i: (Cell::new(false), false, $def, $stb),
                    )+
                }
            }
        }
    )
}

pub fn is_stable_option_and_value<T>(
    option_name: &str,
    option_stable: bool,
    option_value: &T,
) -> bool
where
    T: PartialEq + std::fmt::Debug + ConfigType,
{
    let nightly = false;
    let variant_stable = option_value.stable_variant();
    match (nightly, option_stable, variant_stable) {
        // Stable with an unstable option
        (false, false, _) => {
            tracing::info!(
                "Warning: can't set `{option_name} = {option_value:?}`, unstable features are only \
                       available in nightly channel."
            );
            false
        }
        // Stable with a stable option, but an unstable variant
        (false, true, false) => {
            tracing::info!(
                "Warning: can't set `{option_name} = {option_value:?}`, unstable variants are only \
                       available in nightly channel."
            );
            false
        }
        // Nightly: everything allowed
        // Stable with stable option and variant: allowed
        (true, _, _) | (false, true, true) => true,
    }
}
