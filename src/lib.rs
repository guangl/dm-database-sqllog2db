//! Stream `DaMeng` SQL logs through filtering and parameter substitution into export backends.
//!
//! [`cli`] handles commands; [`engine`] orchestrates exports; [`pipeline`] processes records;
//! [`exporter`] owns output formats. [`stats`] provides SQL analysis.

pub mod application;
pub mod cli;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod stats;

// Keep the established public paths stable while the implementation follows the
// application/domain/infrastructure layout.
pub(crate) use application::scanner;
pub use application::{engine, preflight};
pub use domain::{model, pipeline};
pub use infrastructure::error;
pub(crate) use infrastructure::input;
pub use infrastructure::logging;
pub use infrastructure::output as exporter;
