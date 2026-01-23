// Lightweight logging wrapper.
// - In debug builds we forward to `tracing` for useful output.
// - In release builds we provide no-op stubs to avoid pulling tracing into the binary.

// Export logging macros. When feature `logging` is enabled and we're in debug builds
// these macros forward to `tracing` macros. Otherwise they become no-ops so that
// `tracing` isn't pulled into release binaries.

#[cfg(all(feature = "logging", debug_assertions))]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { tracing::error!($($arg)*); };
}

#[cfg(not(all(feature = "logging", debug_assertions)))]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {};
}

#[cfg(all(feature = "logging", debug_assertions))]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { tracing::warn!($($arg)*); };
}

#[cfg(not(all(feature = "logging", debug_assertions)))]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

#[cfg(all(feature = "logging", debug_assertions))]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { tracing::info!($($arg)*); };
}

#[cfg(not(all(feature = "logging", debug_assertions)))]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[cfg(all(feature = "logging", debug_assertions))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { tracing::debug!($($arg)*); };
}

#[cfg(not(all(feature = "logging", debug_assertions)))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {};
}
