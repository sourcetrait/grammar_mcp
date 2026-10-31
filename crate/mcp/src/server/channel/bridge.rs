//! The running face's Channel, for the code not yet handed it.
use crate::*;

/// The Channel of the face started last, until each reader is handed its own.
static HOST_CHANNEL: std::sync::RwLock<Option<HostChannel>> = std::sync::RwLock::new(None);

/// The Channel's handle and hub as the code without a face reaches them.
#[derive(Clone)]
pub(crate) struct HostChannel {
    pub(crate) channel: Arc<ChannelHandle>,
    pub(crate) hub: Arc<HubSlot>,
    /// The remote links' relays and reports, emitted in their order.
    posts: tk::UnboundedSender<Post>,
}

/// One packet a remote link emits onto the Channel.
pub(crate) struct Post {
    pub(crate) emit: EmitRequest,
    /// The delivery a failed relay is logged under; none for a report.
    pub(crate) delivery: Option<String>,
}

impl HostChannel {
    /// Emit `post` behind those posted before it, a relay's failure logged.
    pub(crate) fn post(
        &self,
        post: Post,
    ) {
        let _ = self.posts.send(post);
    }
}

/// The running face's Channel, none before a face starts.
pub(crate) fn host_channel() -> Option<HostChannel> {
    HOST_CHANNEL.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Make a face's Channel the one the bridge's readers reach.
pub(crate) fn set_host_channel(
    channel: &Arc<ChannelHandle>,
    hub: &Arc<HubSlot>,
) {
    let (posts, queued) = tk::unbounded_channel();
    tk::spawn(drain_posts(channel.clone(), queued));
    let host = HostChannel {
        channel: channel.clone(),
        hub: hub.clone(),
        posts,
    };
    *HOST_CHANNEL.write().unwrap_or_else(|e| e.into_inner()) = Some(host);
}

/// Emit each post in order; a relay that fails is logged by its delivery.
async fn drain_posts(
    channel: Arc<ChannelHandle>,
    mut queued: tk::UnboundedReceiver<Post>,
) {
    while let Some(Post { emit, delivery }) = queued.recv().await {
        let (kind, message) = match channel.request(emit).await {
            Ok(Ok(_)) => continue,
            Ok(Err(EmitError::Refused(e))) => ("channel", e.message().to_string()),
            Ok(Err(EmitError::Render(e))) => ("render", e),
            Err(e) => ("channel", e),
        };
        if let Some(id) = delivery {
            eprintln!("grammar: remote delivery `{id}` not relayed onto the Channel: {kind}: {message}");
        }
    }
}
