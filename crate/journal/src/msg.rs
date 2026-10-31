//! What the journal's controller asks of it, and each request's answer.
use crate::*;

/// A ticket asked for under the lease's epoch, which runs nothing.
#[cereal::derived(Data, Copy, Eq)]
pub struct IssueRequest {
    pub epoch: LeaseEpoch,
}

/// A submission under the lease's epoch, and the most bytes its records may take.
#[cereal::derived(Data)]
pub struct SubmitRequest {
    pub submission: Box<Submission>,
    pub epoch: LeaseEpoch,
    pub reserve: u64,
}

/// An admitted operation's one run: its id and the engine nonce it runs under.
#[cereal::derived(Data, Copy, Eq)]
pub struct Grant {
    pub operation: OperationId,
    pub nonce: datum::Nonce,
}

/// A submission's answer: its admission, its run, a finished one's outcome.
#[cereal::derived(Data)]
pub struct Submitted {
    pub admission: Admission,
    pub grant: Option<Grant>,
    /// The stored outcome a duplicate of a finished operation is answered.
    pub response: Option<Box<Response>>,
}

/// A move short of an operation's end.
#[cereal::derived(Data, Copy, Eq)]
pub enum Step {
    /// The operation given to the engine.
    Start,
    /// The operation's cancellation asked for.
    RequestCancel,
}

impl Step {
    /// The model's event the step is.
    pub(crate) fn event(self) -> OperationEvent {
        match self {
            Self::Start => OperationEvent::Dispatch,
            Self::RequestCancel => OperationEvent::RequestCancel,
        }
    }
}

/// An operation's end.
#[cereal::derived(Data, Copy, Eq)]
pub enum Ending {
    Succeed,
    Fail,
    /// Ended by its cancellation, the worker having seen it end.
    Cancelled,
}

impl Ending {
    /// The model's event the ending is.
    pub(crate) fn event(self) -> OperationEvent {
        match self {
            Self::Succeed => OperationEvent::Succeed,
            Self::Fail => OperationEvent::Fail,
            Self::Cancelled => OperationEvent::Cancelled,
        }
    }
}

/// An admitted operation's step recorded.
#[cereal::derived(Data, Copy, Eq)]
pub struct AdvanceRequest {
    pub operation: OperationId,
    pub step: Step,
}

/// An operation's end recorded with the outcome its response carries.
#[cereal::derived(Data)]
pub struct FinishRequest {
    pub operation: OperationId,
    pub ending: Ending,
    pub outcome: Box<Result<RuntimeReply, TransportError>>,
}

/// What is known of an operation, which runs nothing.
#[cereal::derived(Data, Copy, Eq)]
pub struct LookupRequest {
    pub operation: OperationId,
}

/// What the journal's controller asks of it.
#[cereal::derived(Data)]
pub enum ToJournalSys {
    Issue(IssueRequest),
    Submit(SubmitRequest),
    Advance(AdvanceRequest),
    Finish(FinishRequest),
    Lookup(LookupRequest),
}

/// Each request's answer.
#[cereal::derived(Data)]
pub enum FromJournalSys {
    Issued(Result<OperationTicket, TransportError>),
    Submitted(Submitted),
    Moved(Result<OperationState, StepError>),
    Status(OperationStatus),
}

/// A request's way into `ToJournalSys`.
macro_rules! into_to_sys {
    ($($ty:ty => $variant:ident),* $(,)?) => {$(
        impl From<$ty> for ToJournalSys {
            fn from(v: $ty) -> Self { Self::$variant(v) }
        }
    )*};
}

into_to_sys! {
    IssueRequest => Issue,
    SubmitRequest => Submit,
    AdvanceRequest => Advance,
    FinishRequest => Finish,
    LookupRequest => Lookup,
}

/// An answer's type, taken back out of `FromJournalSys`.
macro_rules! answer_of {
    ($($ty:ty => $variant:ident),* $(,)?) => {$(
        impl TryFrom<FromJournalSys> for $ty {
            type Error = subsys::SubsysError;

            fn try_from(v: FromJournalSys) -> subsys::SubsysResult<Self> {
                match v {
                    FromJournalSys::$variant(answer) => Ok(answer),
                    _ => Err(subsys::SubsysError::ResponseType),
                }
            }
        }
    )*};
}

answer_of! {
    Result<OperationTicket, TransportError> => Issued,
    Submitted => Submitted,
    Result<OperationState, StepError> => Moved,
    OperationStatus => Status,
}

/// Each request's answer type.
macro_rules! request_of {
    ($($ty:ty => $answer:ty),* $(,)?) => {$(
        impl subsys::Request<JournalSystem> for $ty {
            type ResponseType = $answer;
        }
    )*};
}

request_of! {
    IssueRequest => Result<OperationTicket, TransportError>,
    SubmitRequest => Submitted,
    AdvanceRequest => Result<OperationState, StepError>,
    FinishRequest => Result<OperationState, StepError>,
    LookupRequest => OperationStatus,
}
