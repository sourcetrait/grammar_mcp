//! The running face's systems, for the code not yet handed them.
use crate::*;

/// The systems of the face started last, until each reader is handed its own.
static HOST_CHANNEL: std::sync::RwLock<Option<HostChannel>> = std::sync::RwLock::new(None);

/// The face's handles and hub, as the code without a face reaches them.
#[derive(Clone)]
pub(crate) struct HostChannel {
    pub(crate) channel: Arc<ChannelHandle>,
    pub(crate) peer: Arc<PeerHandle>,
    pub(crate) hub: Arc<HubSlot>,
}

/// The running face's systems, none before a face starts.
pub(crate) fn host_channel() -> Option<HostChannel> {
    HOST_CHANNEL.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Make a face's systems the ones the bridge's readers reach.
pub(crate) fn set_host_channel(
    channel: &Arc<ChannelHandle>,
    peer: &Arc<PeerHandle>,
    hub: &Arc<HubSlot>,
) {
    let host = HostChannel {
        channel: channel.clone(),
        peer: peer.clone(),
        hub: hub.clone(),
    };
    *HOST_CHANNEL.write().unwrap_or_else(|e| e.into_inner()) = Some(host);
}
