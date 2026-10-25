use crate::*;

/// Liquid templating
pub struct SoakPlugin;

impl nu_plugin::Plugin for SoakPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(SoakDirCommand),
            Box::new(SoakStrCommand),
            Box::new(StrSoakCommand),
        ]
    }
}
