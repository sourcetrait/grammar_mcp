fn main() {
    nu_plugin::serve_plugin(&nu_plugin_soak::SoakPlugin, nu_plugin::MsgPackSerializer {});
}
