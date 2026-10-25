//! nushell's gstat plugin, upstream's main (MIT) over its library.

/// Serves the gstat plugin to its engine over stdio, in MessagePack.
fn main() {
    nu_plugin::serve_plugin(&nu_plugin_gstat::GStatPlugin, nu_plugin::MsgPackSerializer {})
}
