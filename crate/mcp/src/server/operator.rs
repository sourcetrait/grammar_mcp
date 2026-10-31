//! The operator's handler and observer: its emergencies routed, the engine seen.
use crate::*;

/// The operator's emergencies sent up; its status and end to mcp's loop.
pub(crate) struct OperatorWatch {
    /// mcp's system's way up, which carries each emergency to its controller's policy.
    pub up: tk::Sender<subsys::MsgFromSys<FromMcpSys>>,
    pub notices: tk::UnboundedSender<Notice>,
}

impl subsys::Handler<OperatorSystem> for OperatorWatch {
    fn on_packet(&mut self, pkt: subsys::Packet<FromOperatorSys>) -> impl Future<Output = Option<subsys::Packet<FromOperatorSys>>> + 'static + Send {
        let up = self.up.clone();
        async move {
            let FromOperatorSys::Emergency(emergency) = pkt.take_msg();
            let packet = subsys::Packet::singular(FromMcpSys::Emergency(emergency));
            let _ = up.send(subsys::MsgFromSys::Packet(packet)).await;
            None
        }
    }

    fn on_sub(&mut self, sub: subsys::FromSub) -> impl Future<Output = ()> + Send {
        notice_status(&self.notices, "operator", sub);
        std::future::ready(())
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        let _ = self.notices.send(Notice::Closed { component: "operator" });
        std::future::ready(())
    }
}

/// The engine's health, or none when it doesn't answer within a sample's interval.
async fn engine_health(engine: &EngineHandle) -> Option<EngineHealth> {
    match tk::timeout(SAMPLE_INTERVAL, engine.control(EngineCommand::Health)).await {
        Ok(Ok(ControlAnswer::Health(health))) => Some(health),
        _ => None,
    }
}

/// Whether the running face's Channel is open; with no face, it is closed.
async fn channel_open() -> bool {
    let Some(host) = host_channel() else {
        return false;
    };
    match host.channel.request(StatusRequest).await {
        Ok(status) => !matches!(status.state.phase, ChannelPhase::Closed),
        Err(_) => false,
    }
}

/// Each interval, pass the operator the engine's health and the Channel's state.
pub(crate) fn spawn_observer(
    engine: Arc<EngineHandle>,
    operator: Arc<OperatorHandle>,
) -> tk::JoinHandle<()> {
    tk::spawn(async move {
        loop {
            tk::sleep(SAMPLE_INTERVAL).await;
            let reading = engine_health(&engine).await;
            let listening = channel_open().await;
            let observation = Observation {
                engine: reading,
                listening,
            };
            if let Err(e) = operator.observe(observation).await {
                eprintln!("{e}");
                return;
            }
        }
    })
}
