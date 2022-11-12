// TODO: remove this file
pub use tonic_sdk_macros_debug as debug;

/// Activate with the `debug_log` feature.
#[macro_export]
macro_rules! debug_log {
    ($($x:expr),+) => {
        #[cfg(feature = "debug_log")]
        near_sdk::log!("{}:{} {}", file!(), line!(), format!($($x),+))
    };
}

/// Return storage usage increase due to this block
#[macro_export]
macro_rules! measure_storage_increase {
    ($body: block) => {{
        let before = near_sdk::env::storage_usage();

        {
            $body
        }

        let after = near_sdk::env::storage_usage();

        if before > after {
            near_sdk::env::panic_str("storage decreased, expected increase");
        }

        after - before
    }};
}

/// Intended for Option-based fields that are borsh-skipped and must be
/// initialized at runtime. Assumes that `field` is an [Option].
#[macro_export]
macro_rules! impl_lazy_accessors {
    ($field:ident, $getter:ident, $setter:ident, $t:ty) => {
        pub fn $getter(&self) -> $t {
            _expect!(
                self.$field,
                &format!("unitialized field {}", stringify!($field))
            )
        }

        pub fn $setter(&mut self, v: $t) {
            if let Some(_) = self.$field.replace(v) {
                // not a bug, more like a cache hit
                debug_log!("field {} already initialized", stringify!($field));
            }
        }
    };
}

/// Intended for Option-based fields that are borsh-skipped and must be
/// initialized at runtime. Assumes that `field` is an [Option].
#[macro_export]
macro_rules! impl_lazy_accessors_clone {
    ($field:ident, $getter:ident, $setter:ident, $t:ty) => {
        pub fn $getter(&self) -> $t {
            _expect!(
                self.$field.clone(),
                &format!("unitialized field {}", stringify!($field))
            )
        }

        pub fn $setter(&mut self, v: $t) {
            if let Some(_) = self.$field.replace(v) {
                // not a bug, more like a cache hit
                debug_log!("field {} already initialized", stringify!($field));
            }
        }
    };
}

/// Replacement for `.expect` that panics with [near_sdk::env::panic_str].
///
/// Reduces the compiled binary size and provides cleaner error output for
/// expected runtime errors.
#[macro_export]
macro_rules! _expect {
    ($option:expr, $msg:expr) => {
        $option.unwrap_or_else(|| near_sdk::env::panic_str($msg))
    };
    ($option:expr, $field:ident, $msg:expr) => {
        $option
            .$field
            .unwrap_or_else(|| near_sdk::env::panic_str($msg))
    };
}

/// Replacement for `assert!` that panics with [near_sdk::env::panic_str].
///
/// Reduces the compiled binary size and provides cleaner error output for
/// expected runtime errors.
#[macro_export]
macro_rules! _assert {
    ($condition:expr, $msg:expr) => {
        if !($condition) {
            near_sdk::env::panic_str($msg)
        }
    };
}

/// Replacement for `assert_eq!` that panics with [near_sdk::env::panic_str].
///
/// Reduces the compiled binary size and provides cleaner error output for
/// expected runtime errors.
#[macro_export]
macro_rules! _assert_eq {
    ($left:expr, $right:expr, $msg:expr) => {
        _assert!(($left) == ($right), $msg)
    };
}

/// Replacement for `assert_ne!` that panics with [near_sdk::env::panic_str].
///
/// Reduces the compiled binary size and provides cleaner error output for
/// expected runtime errors.
#[macro_export]
macro_rules! _assert_ne {
    ($left:expr, $right:expr, $msg:expr) => {
        _assert!(($left) != ($right), $msg)
    };
}
