use crate::*;

impl subsys::Request<EngineSystem> for ExecuteRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<EngineSystem> for RerunRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<EngineSystem> for CallRequest {
    type ResponseType = ValueAnswer;
}

impl subsys::Request<EngineSystem> for ControlRequest {
    type ResponseType = EngineResult<ControlAnswer>;
}
