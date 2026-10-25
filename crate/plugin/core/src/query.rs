//! nushell's query plugin, upstream's main (MIT) over its library.

/// Serves the query plugin to its engine over stdio, in JSON.
fn main() {
    nu_plugin::serve_plugin(&nu_plugin_query::Query {}, nu_plugin::JsonSerializer {})
}
