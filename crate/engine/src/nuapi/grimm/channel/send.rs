use crate::*;

/// `grimm channel send <model> <event> [attached]` - the state-update lane.
#[derive(Clone)]
pub(crate) struct GrimmChannelSend;

impl nu::Command for GrimmChannelSend {
    fn name(&self) -> &str { Self::DEF.name }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build(Self::DEF.name)
            .vocab(&Self::DEF)
            .vocab_required(&Self::DEF_MODEL, nu::SyntaxShape::String)
            .vocab_required(&Self::DEF_EVENT, data_shape())
            .vocab_optional(&Self::DEF_ATTACHED, data_shape())
            .input_output_types(vec![(nu::Type::Nothing, event_id_type())])
    }

    fn examples(&self) -> Vec<nu::Example<'_>> { Self::DEF.examples() }

    fn description(&self) -> &str { Self::DEF.description }

    fn run(
        &self,
        engine_state: &nu::EngineState,
        stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        let model: String = call.req(engine_state, stack, 0)?;
        let event: nu::Value = call.req(engine_state, stack, 1)?;
        require_record_or_table(&event, call.head)?;
        let attached: Option<nu::Value> = call.opt(engine_state, stack, 2)?;
        if let Some(value) = &attached {
            require_record_or_table(value, call.head)?;
        }
        let from = NuapiCall::current(engine_state, stack, call.head)?.origin();
        let id = host()
            .channel_send(ChannelSend {
                from,
                model,
                event,
                attached,
                stop: &|| stop_offender(engine_state),
            })
            .map_err(|message| shell_error(nu::Command::name(self), &message, call.head))?;
        Ok(nu::PipelineData::Value(event_id_record(id, call.head), None))
    }
}

/// A refused send: the command titles it, and the host's reason labels it.
fn shell_error(
    command: &str,
    message: &str,
    span: nu::Span,
) -> nu::ShellError {
    nu::GenericError::new(
        format!("{command} failed"),
        message.to_string(),
        span,
    )
    .into()
}

/// The root job: a foreground eval, owning no entry in the jobs table.
const ROOT_JOB_ID: usize = 0;

/// Stop the origin that just crossed the hard threshold.
fn stop_offender(engine_state: &nu::EngineState) -> String {
    engine_state.signals().trigger();
    let job_id = engine_state.current_job.id;
    if job_id.get() == ROOT_JOB_ID {
        return "signals triggered".to_string();
    }
    let mut jobs = engine_state.jobs.lock().unwrap_or_else(|e| e.into_inner());
    match jobs.kill_and_remove(job_id) {
        Ok(()) => format!("signals triggered; job {job_id} killed"),
        Err(e) => format!("signals triggered; job {job_id} removed, kill incomplete: {e}"),
    }
}
