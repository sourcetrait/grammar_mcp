//! The remote engines' handler: their workers' asks and emergencies, and their end.
use crate::*;

/// The controller's handler of the remote engines' system.
#[derive(Clone)]
pub(crate) struct RemoteRoute {
    /// The controller's shutdown flag, under which the system's end goes unsaid.
    pub(crate) stopping: Arc<AtomicBool>,
}

impl subsys::Handler<RemoteSystem> for RemoteRoute {
    fn on_packet(&mut self, pkt: subsys::Packet<FromRemote>) -> impl Future<Output = Option<subsys::Packet<FromRemote>>> + 'static + Send {
        if let FromRemote::Emergency(RemoteEmergency { alias, emergency }) = pkt.msg() {
            eprintln!("grammar: {alias}: the worker's emergency {}, which the MCP holds", emergency.kind().name());
        }
        std::future::ready(None)
    }

    fn on_request(&mut self, pkt: subsys::Packet<FromRemote>) -> impl Future<Output = Option<ToRemote>> + 'static + Send {
        let answer = match pkt.take_msg() {
            FromRemote::Ask(RemoteAsk { alias, .. }) => Some(ToRemote::Answer(HostAnswer::Refused(format!(
                "the MCP answers no ask of the remote engine `{alias}`"
            )))),
            _ => None,
        };
        std::future::ready(answer)
    }

    fn on_sub(&mut self, sub: subsys::FromSub) -> impl Future<Output = ()> + Send {
        log_status("remote engines", sub, &self.stopping);
        std::future::ready(())
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        log_closed("remote engines", &self.stopping);
        std::future::ready(())
    }
}
