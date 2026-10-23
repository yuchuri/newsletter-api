use std::{env, io};

use tracing_subscriber::{
    EnvFilter,
    fmt::{format::FmtSpan, writer::BoxMakeWriter},
};

pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_span_events(FmtSpan::CLOSE)
        .json()
        .init();
}

pub fn init_test() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_span_events(FmtSpan::CLOSE)
        .pretty()
        .with_writer(if env::var("TEST_LOG").is_ok() {
            BoxMakeWriter::new(io::stdout)
        } else {
            BoxMakeWriter::new(io::sink)
        })
        .try_init()
        .ok();
}
