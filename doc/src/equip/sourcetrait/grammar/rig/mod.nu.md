# rig/mod.nu

The grammar builtin's rig, called as `grammar:<module>:<call>`: the essentials,
one call a plugin command the MCP bundles, `eye dir`, `eye git`, and `soak dir`.
Each call passes its arguments through unchanged, so the plugin's own help is
the call's whole behaviour.

`soak dir` takes its values as an open `record<>`, since a template names
whatever fields it likes, and returns nothing, as the command does. The two
`eye` calls return their tree in `{tree}`, a call answering a record.
