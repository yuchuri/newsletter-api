use std::{
    error::Error,
    fmt::{self, Formatter},
};

mod confirmation_error;
mod publish_error;
mod subscribe_error;

pub use confirmation_error::*;
pub use publish_error::*;
pub use subscribe_error::*;

fn error_chain_fmt(e: &impl Error, f: &mut Formatter<'_>) -> fmt::Result {
    writeln!(f, "{e}\n")?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{cause}")?;
        current = cause.source();
    }
    Ok(())
}
