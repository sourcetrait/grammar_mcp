use crate::*;

/// Parameters for `interface equip scaffold`.
#[derive(Debug, ser::Deserialize, ser::Serialize)]
pub struct NewParams {
    /// Namepaths to scaffold into installed equips' rigs: a module or a function.
    pub namepaths: Vec<String>,
}

/// Success result of `interface equip scaffold`: every path the batch made.
#[cfg(feature = "embedded-cli")]
#[derive(Debug, ser::Serialize)]
pub(crate) struct NewResult {
    pub created: Vec<String>,
}

#[cfg(feature = "embedded-cli")]
impl NuSh {
    /// `interface equip scaffold`: modules and calls scaffolded into rigs.
    pub(crate) async fn equip_scaffold(
        &self,
        nonce: datum::Nonce,
        p: NewParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let command = EngineCommand::EquipScaffold {
            namepaths: p.namepaths,
        };
        match self.system.control(command).await {
            Ok(ControlAnswer::Created(created)) => success(nonce, &NewResult { created }),
            Ok(_) => error_to_call_result(unexpected("equip_scaffold"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }
}
