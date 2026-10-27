use crate::*;

/// Parameters for `interface process kill`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct KillParams {
    /// The nonce of the in-flight call to cancel.
    pub nonce: String,
}

impl NuSh {
    /// `interface process kill`: cancel an in-flight call by its nonce.
    pub(crate) async fn process_kill(
        &self,
        nonce: datum::Nonce,
        p: KillParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let found = {
            let map = self.in_flight.lock().await;
            match map.get(&p.nonce) {
                Some(entry) => {
                    entry.cancel.store(true, Ordering::SeqCst);
                    let lane = entry.kind.lane();
                    register_hung(
                        &self.hung_watch,
                        HungWatch {
                            nonce: p.nonce.clone(),
                            tool: entry.tool,
                            lane,
                            started_at: entry.started_at,
                            cancelled_at: now_millis(),
                            finished: entry.finished.clone(),
                        },
                    );
                    Some(entry.tracker.collect_pids())
                }
                None => None,
            }
        };
        if let Some(pids) = found {
            tree_kill(&pids);
            kill_plugin_subprocesses();
        }
        success_void(nonce)
    }
}
