#![allow(clippy::bool_comparison, clippy::enum_variant_names)]

pub mod application;
pub mod jwt;
pub mod routes;
pub mod settings;
pub mod telemetry;
pub mod websocket;

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
