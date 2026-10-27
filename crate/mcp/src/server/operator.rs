//! The operator's handler and observer: its emergencies routed, the engine seen.
use crate::*;

/// The operator's emergencies onto the lane; its status and end to mcp's loop.
pub(crate) struct OperatorWatch {
    pub emergencies: EmergencyTx,
    pub notices: tk::UnboundedSender<Notice>,
}

impl subsys::Handler<OperatorSystem> for OperatorWatch {
    fn on_packet(&mut self, pkt: subsys::Packet<FromOperatorSys>) -> impl Future<Output = Option<subsys::Packet<FromOperatorSys>>> + 'static + Send {
        let FromOperatorSys::Emergency(emergency) = pkt.take_msg();
        let _ = self.emergencies.send(emergency);
        std::future::ready(None)
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

/// The engine's health as the operator reads it.
fn engine_reading(health: EngineHealth) -> EngineReading {
    let hung = health
        .hung
        .into_iter()
        .map(|eval| HungThread {
            nonce: eval.nonce,
            tool: eval.tool,
            lane: match eval.lane {
                Lane::Stateless => ThreadLane::Stateless,
                Lane::Stateful => ThreadLane::Stateful,
            },
            started_at: eval.started_at,
            cancelled_at: eval.cancelled_at,
        })
        .collect();
    EngineReading {
        hung,
        permits_held: health.permits_held,
        permits_cap: health.permits_cap,
        jobs: health.jobs,
    }
}

/// The engine's health, or none when it doesn't answer within a sample's interval.
async fn engine_health(engine: &EngineHandle) -> Option<EngineHealth> {
    match tk::timeout(SAMPLE_INTERVAL, engine.control(EngineCommand::Health)).await {
        Ok(Ok(ControlAnswer::Health(health))) => Some(health),
        _ => None,
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
            let reading = engine_health(&engine).await.map(engine_reading);
            let listening = !matches!(channel_handle().status().phase, ChannelPhase::Closed);
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
