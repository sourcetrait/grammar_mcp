//! The running face's Channel, for the code not yet handed it.
use crate::*;

/// The Channel of the face started last, until each reader is handed its own.
static HOST_CHANNEL: std::sync::RwLock<Option<HostChannel>> = std::sync::RwLock::new(None);

/// The face's Channel handle, as the code without a face reaches it.
#[derive(Clone)]
pub(crate) struct HostChannel {
    pub(crate) channel: Arc<ChannelHandle>,
}

/// The running face's Channel, none before a face starts.
pub(crate) fn host_channel() -> Option<HostChannel> {
    HOST_CHANNEL.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Make a face's Channel the one the bridge's readers reach.
pub(crate) fn set_host_channel(channel: &Arc<ChannelHandle>) {
    let host = HostChannel {
        channel: channel.clone(),
    };
    *HOST_CHANNEL.write().unwrap_or_else(|e| e.into_inner()) = Some(host);
}
