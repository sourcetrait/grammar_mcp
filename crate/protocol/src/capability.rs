//! The capabilities a build advertises, each off until its gate passes.
use crate::*;

/// What a build of the daemon's packages can do, as its hello advertises it.
#[cereal::derived(Data, Copy, Eq)]
pub struct Capabilities {
    /// The bounded v1 framing and its messages.
    pub wire: bool,
    /// The persistent worker's runtime.
    pub runtime: bool,
    /// The native SSH client.
    pub client: bool,
    /// Engine artifacts over SFTP.
    pub artifacts: bool,
    /// The user service and its socket activation.
    pub service: bool,
}

impl Capabilities {
    /// No capability on, what a build advertises before its gates pass.
    pub const NONE: Self = Self {
        wire: false,
        runtime: false,
        client: false,
        artifacts: false,
        service: false,
    };

    /// The capability a hello's required feature names, if it names one.
    pub fn named(&self, feature: &str) -> Option<bool> {
        match feature {
            "wire" => Some(self.wire),
            "runtime" => Some(self.runtime),
            "client" => Some(self.client),
            "artifacts" => Some(self.artifacts),
            "service" => Some(self.service),
            _ => None,
        }
    }
}
