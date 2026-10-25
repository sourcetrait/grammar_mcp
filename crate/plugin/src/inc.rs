//! nushell's inc plugin, upstream's main (MIT) over its library.

/// Serves the inc plugin to its engine over stdio, in JSON.
fn main() {
    nu_plugin::serve_plugin(&nu_plugin_inc::IncPlugin, nu_plugin::JsonSerializer {})
}
