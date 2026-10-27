use crate::*;

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
