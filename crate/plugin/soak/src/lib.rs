pub(crate) mod cork {
    pub(crate) mod consts;
    pub(crate) mod error;
    pub(crate) mod fs;
}
pub(crate) mod error;
pub(crate) mod plugin;
pub(crate) mod render;
pub(crate) mod soak_dir;
pub(crate) mod soak_str;
pub(crate) mod str_soak;

pub(crate) use crate::cork::{
    error::{CorkError, ErrorTrait, ErrorMeta, PathKind, PathOp},
    fs::{PathSpan},
    consts::{NOSPAN},
};

pub(crate) use crate::{
    error::{SoakError, SoakResult},
    render::{render_dir, render_str},
    soak_dir::SoakDirCommand,
    soak_str::SoakStrCommand,
    str_soak::StrSoakCommand,
};

pub use crate::plugin::SoakPlugin;

pub(crate) use std::{
    ffi::OsString,
    fmt::Display,
    fs::{self, File},
    path::{PathBuf, Path},
    io::{self, BufWriter},
    time::Duration,
};

pub(crate) use nu_ansi_term::Color;
