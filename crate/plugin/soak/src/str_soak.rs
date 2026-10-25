use crate::*;

pub(crate) struct StrSoakCommand;

const STR_SOAK: &str = "str soak";

impl nu_plugin::SimplePluginCommand for StrSoakCommand {
    type Plugin = SoakPlugin;

    fn name(&self) -> &str {
        STR_SOAK
    }
    fn description(&self) -> &str {
        "Render a piped Liquid template string using a record of values."
    }

    fn signature(&self) -> nu_protocol::Signature {
        nu_protocol::Signature::build(STR_SOAK)
            .required(
                "fill",
                nu_protocol::SyntaxShape::Record(vec![].into()),
                "Values applied to the template",
            )
            .input_output_types(vec![(nu_protocol::Type::String, nu_protocol::Type::String)])
            .category(nu_protocol::Category::Formats)
    }

    fn examples(&self) -> Vec<nu_protocol::Example<'_>> {
        vec![nu_protocol::Example {
            example: r#"'Hello, {{ name }}!' | str soak { name: World }"#,
            description: "Render a Liquid string against a fill record.",
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
        let fill = call.req::<nu_protocol::Value>(0)?;
        render_str(input.as_str()?, fill.as_record()?)
            .map_err(|e| e.nu(call.head))
    }
}
