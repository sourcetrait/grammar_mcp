//! What this build of the client can do.
use crate::*;

/// The client's capabilities, none on before its gates pass.
pub fn capabilities() -> Capabilities {
    Capabilities::NONE
}
