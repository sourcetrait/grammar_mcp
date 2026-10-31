//! The session's current purview, the ids it has in view, resolved by the engine.
use crate::*;

/// What changed between two pattern sets, as `(added, removed)`.
pub(crate) fn pattern_delta(
    before: &[String],
    after: &[String],
) -> (Vec<String>, Vec<String>) {
    let added: Vec<String> = after
        .iter()
        .filter(|p| !before.contains(p))
        .cloned()
        .collect();
    let removed: Vec<String> = before
        .iter()
        .filter(|p| !after.contains(p))
        .cloned()
        .collect();
    (added, removed)
}

/// The current purview: which ids this host process has in view.
pub(crate) struct CurrentPurview {
    ids: std::sync::Mutex<Vec<String>>,
}

impl Default for CurrentPurview {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentPurview {
    /// A fresh view, `default` alone.
    pub(crate) fn new() -> Self {
        Self {
            ids: std::sync::Mutex::new(vec![PURVIEW_DEFAULT.to_string()]),
        }
    }

    /// The view's lock, a poisoned one taken as it stands.
    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        self.ids.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The ids in view.
    pub(crate) fn ids(&self) -> Vec<String> {
        self.lock().clone()
    }

    /// Add ids to the current view; additive and order-preserving.
    pub(crate) fn extend(
        &self,
        add: &[String],
    ) -> Vec<String> {
        let mut ids = self.lock();
        for id in add {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids.clone()
    }

    /// Replace what is in view; an empty list means `default`.
    pub(crate) fn set(
        &self,
        want: &[String],
    ) -> Vec<String> {
        let mut ids = self.lock();
        *ids = if want.is_empty() {
            vec![PURVIEW_DEFAULT.to_string()]
        } else {
            want.to_vec()
        };
        ids.clone()
    }

    /// Drop ids the rows no longer hold.
    pub(crate) fn retain_known(
        &self,
        rows: &[PurviewRow],
    ) {
        let mut ids = self.lock();
        ids.retain(|id| id == PURVIEW_ALL || rows.iter().any(|row| &row.id == id));
        if ids.is_empty() {
            *ids = vec![PURVIEW_DEFAULT.to_string()];
        }
    }
}

/// The refusal of a view naming an id no purview answers to.
pub(crate) fn unknown_purview(id: &str) -> GrammarMcpError {
    GrammarMcpError::PurviewInvalidId {
        id: id.to_string(),
        reason: "no such purview; configure it first".to_string(),
    }
}

impl NuSh {
    /// What a set of ids puts in view, as the engine resolves it.
    pub(crate) async fn resolve_purviews(
        &self,
        ids: Vec<String>,
    ) -> GrammarMcpResult<PurviewResolution> {
        Ok(self.system.request(PurviewResolveRequest { ids }).await??)
    }
}
