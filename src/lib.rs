//! Stream `DaMeng` SQL logs through filtering and parameter substitution into export backends.
//!
//! [`cli`] handles commands; [`engine`] orchestrates exports; [`pipeline`] processes records;
//! [`exporter`] owns output formats. [`stats`] provides SQL analysis.

pub mod cli;
pub mod config;
pub mod engine;
pub mod error;
pub mod exporter;
pub mod logging;
pub(crate) mod parser;
pub mod pipeline;
pub mod preflight;
pub(crate) mod scanner;
pub mod stats;
pub(crate) mod streaming;
