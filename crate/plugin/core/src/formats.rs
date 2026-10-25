//! nushell's formats plugin, upstream's main (MIT) over its library.

/// Serves the formats plugin to its engine over stdio, in MessagePack.
fn main() {
    nu_plugin::serve_plugin(&nu_plugin_formats::FormatCmdsPlugin, nu_plugin::MsgPackSerializer {})
}
