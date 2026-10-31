//! The worker's controller: its process's systems, started and stopped in order.
use crate::*;

/// The journal's directory within the instance's durable state.
const JOURNAL_DIR: &str = "operations";

/// The worker's systems, under the one controller its process owns.
pub(crate) struct GrammardController {
    journal: JournalHandle,
}

impl fmt::Debug for GrammardController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GrammardController").finish_non_exhaustive()
    }
}

impl GrammardController {
    /// The worker's systems started on its task runtime, the journal first.
    pub(crate) async fn start(
        config: &InstanceConfig,
        generation: WorkerGeneration,
    ) -> DaemonResult<Self> {
        let root = config.paths.state.join(JOURNAL_DIR);
        let instance = Instance::new(&config.identity.id, &config.identity.namespace).map_err(|e| {
            DaemonError::JournalUnavailable {
                path: root.clone(),
                reason: e.message(),
            }
        })?;
        let params = JournalSysParams {
            generation,
            instance,
            cap: JOURNAL_CAP,
        };
        let journal = JournalHandle::start(root, params, JournalWatch).await.map_err(|e| match e {
            JournalError::Held { path } => DaemonError::JournalHeld { path },
            JournalError::Unavailable { path, reason } => DaemonError::JournalUnavailable { path, reason },
        })?;
        Ok(Self { journal })
    }

    /// The systems stopped in order, each one's files let go with it.
    pub(crate) async fn shutdown(self) {
        if let Err(e) = self.journal.stop().await {
            eprintln!("grammard: {e}");
        }
    }
}

/// The journal's handler: it sends nothing up, and an unasked end is said.
struct JournalWatch;

impl subsys::Handler<JournalSystem> for JournalWatch {
    fn on_packet(
        &mut self,
        _pkt: subsys::Packet<FromJournalSys>,
    ) -> impl Future<Output = Option<subsys::Packet<FromJournalSys>>> + 'static + Send {
        std::future::ready(None)
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        eprintln!("grammard: the operation journal's system ended with no stop asked");
        std::future::ready(())
    }
}
