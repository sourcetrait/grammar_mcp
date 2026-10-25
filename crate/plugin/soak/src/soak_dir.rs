use std::path::Path;

use crate::*;

/// Render a directory of Liquid templates using a piped record of values
pub(crate) struct SoakDirCommand;

const SOAK_DIR: &str = "soak dir";

impl nu_plugin::SimplePluginCommand for SoakDirCommand {
    type Plugin = SoakPlugin;

    fn name(&self) -> &str {
        SOAK_DIR
    }
    fn description(&self) -> &str {
        "Render a directory of Liquid templates using a piped record of values."
    }

    fn signature(&self) -> nu_protocol::Signature {
        nu_protocol::Signature::build(SOAK_DIR)
            .required(
                "src",
                nu_protocol::SyntaxShape::Directory,
                "Source directory",
            )
            .required(
                "dst",
                nu_protocol::SyntaxShape::Directory,
                "Destination directory",
            )
            .input_output_types(vec![(
                nu_protocol::Type::record(),
                nu_protocol::Type::Nothing,
            )])
            .category(nu_protocol::Category::Formats)
    }

    fn examples(&self) -> Vec<nu_protocol::Example<'_>> {
        vec![nu_protocol::Example {
            example: r#"{ name: World } | soak dir /tmp/input-dir /tmp/output-dir"#,
            description: "Render a directory of Liquid templates against a piped record.",
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &SoakPlugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: &nu_protocol::Value,
    ) -> Result<nu_protocol::Value, nu_protocol::LabeledError> {
        let src = call.req::<nu_protocol::Value>(0)?;
        let dst = call.req::<nu_protocol::Value>(1)?;
        render_dir(
            PathSpan { path: Path::new(src.as_str()?), span: src.span() },
            PathSpan { path: Path::new(dst.as_str()?), span: dst.span() },
            None,
            None,
            None,
            input.as_record()?,
        ).map_err(|e| e.nu(call.head))
    }
}
