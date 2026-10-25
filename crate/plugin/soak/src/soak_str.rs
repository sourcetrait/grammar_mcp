use crate::*;

/// Render a Liquid template string using a piped record of values
pub(crate) struct SoakStrCommand;

const SOAK_STR: &str = "soak str";

impl nu_plugin::SimplePluginCommand for SoakStrCommand {
    type Plugin = SoakPlugin;

    fn name(&self) -> &str {
        SOAK_STR
    }
    fn description(&self) -> &str {
        "Render a Liquid template string using a piped record of values."
    }

    fn signature(&self) -> nu_protocol::Signature {
        nu_protocol::Signature::build(SOAK_STR)
            .required(
                "liquid",
                nu_protocol::SyntaxShape::String,
                "Liquid template",
            )
            .input_output_types(vec![(
                nu_protocol::Type::record(),
                nu_protocol::Type::String,
            )])
            .category(nu_protocol::Category::Formats)
    }

    fn examples(&self) -> Vec<nu_protocol::Example<'_>> {
        vec![nu_protocol::Example {
            example: r#"{ name: World } | soak str 'Hello, {{ name }}!'"#,
            description: "Render a Liquid template against a piped record.",
            result: Some(nu_protocol::Value::string("Hello, World!", NOSPAN)),
        }]
    }

    fn run(
        &self,
        _plugin: &SoakPlugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: &nu_protocol::Value,
    ) -> Result<nu_protocol::Value, nu_protocol::LabeledError> {
        let template = call.req::<nu_protocol::Value>(0)?;
        render_str(template.as_str()?, input.as_record()?)
            .map_err(|e| e.nu(call.head))
    }
}
