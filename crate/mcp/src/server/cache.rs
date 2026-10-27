use crate::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CacheKind {
    Nu,
    Calls,
}

impl CacheKind {
    fn dir_name(self) -> &'static str {
        match self {
            Self::Nu => "nu",
            Self::Calls => "calls",
        }
    }
}

/// The user's base dirs, `None` where no home directory can be found.
pub(crate) static BASE_DIRS: LazyLock<Option<dirs::BaseDirs>> = LazyLock::new(dirs::BaseDirs::new);

/// Startup's check that a home directory can be found.
pub(crate) fn check_base_dirs() -> Result<(), String> {
    match BASE_DIRS.as_ref() {
        Some(_) => Ok(()),
        None => Err("grammar: no home directory can be found; set HOME".to_string()),
    }
}

/// The base dirs, which startup's check has found.
pub(crate) fn base_dirs() -> &'static dirs::BaseDirs {
    BASE_DIRS.as_ref().expect("base dirs checked at startup")
}

pub(crate) fn cache_base_dir() -> PathBuf {
    base_dirs()
        .cache_dir()
        .join(consts::GRAMMAR)
        .join(consts::MCP)
        .join(&config().id)
        .join(&config().namespace)
}

pub(crate) fn data_base_dir() -> PathBuf {
    base_dirs()
        .data_dir()
        .join(consts::GRAMMAR)
        .join(consts::MCP)
        .join(&config().id)
        .join(&config().namespace)
}

pub(crate) fn cache_kind_dir(kind: CacheKind) -> PathBuf {
    cache_base_dir().join(kind.dir_name())
}

pub(crate) fn cache_dir(kind: CacheKind, nonce: &datum::NoncePair) -> PathBuf {
    cache_kind_dir(kind).join(nonce.as_str())
}

/// The log dir of the call `nonce` names, in whichever cache kind holds it.
pub(crate) fn call_log_dir(nonce: &str) -> Option<PathBuf> {
    if nonce.is_empty() || !nonce.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    [CacheKind::Nu, CacheKind::Calls]
        .into_iter()
        .map(|kind| cache_kind_dir(kind).join(nonce))
        .find(|dir| dir.is_dir())
}

/// A `nu` call's definition, cached beside its logs for `rerun`.
pub(crate) const DEFINITION_FILE: &str = "definition.nu";

/// The definition a `nu` call cached, at `nu/<nonce>/definition.nu`.
pub(crate) fn definition_file(nonce: &str) -> PathBuf {
    cache_kind_dir(CacheKind::Nu).join(nonce).join(DEFINITION_FILE)
}
