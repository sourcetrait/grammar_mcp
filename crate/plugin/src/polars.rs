//! nushell's polars plugin, upstream's main (MIT) over its library.

/// Serves the polars plugin to its engine over stdio, in MessagePack.
fn main() {
    env_logger::init();
    unsafe {
        std::env::set_var("POLARS_ALLOW_EXTENSION", "true");
    }
    match nu_plugin_polars::PolarsPlugin::new() {
        Ok(ref plugin) => nu_plugin::serve_plugin(plugin, nu_plugin::MsgPackSerializer {}),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
