pub(crate) mod artifact {}
pub(crate) mod capability;
pub(crate) mod config {}
pub(crate) mod credential {}
pub(crate) mod platform {}
pub(crate) mod ssh {}
pub(crate) mod transport {}
pub(crate) mod trust {}

pub(crate) use sourcetrait_grammar_protocol::Capabilities;

pub use crate::capability::capabilities;
