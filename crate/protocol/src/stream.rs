//! Each stream role's frames after its hello, both ways, and their liveness.
use crate::*;

/// A liveness probe, its nonce echoed in the answer.
#[cereal::derived(Data, Copy, Eq)]
pub struct Ping {
    pub nonce: u64,
}

/// A stream's orderly end, with why its sender ended it where it wasn't asked.
#[cereal::derived(Data)]
pub struct StreamClose {
    pub error: Option<TransportError>,
}

/// What a controller sends on its RPC stream after the hello.
#[cereal::derived(Data)]
pub enum RpcFrame {
    Ticket(TicketRequest),
    Submit(Box<Submission>),
    Status(StatusQuery),
    Lease(LeaseRequest),
    Ping(Ping),
    Close(StreamClose),
}

/// What the worker answers on an RPC stream.
#[cereal::derived(Data)]
pub enum RpcAnswer {
    Ticket(TicketReply),
    Admission(Admission),
    Response(Box<Response>),
    Status(StatusReply),
    Lease(Result<LeaseReply, TransportError>),
    Pong(Ping),
    Close(StreamClose),
}

/// What a subscriber sends on its Events stream after the hello.
#[cereal::derived(Data)]
pub enum EventsFrame {
    /// Where the subscription starts, its first frame.
    Resume(EventResume),
    Ack(EventAck),
    Ping(Ping),
    Close(StreamClose),
}

/// What the worker sends on an Events stream.
#[cereal::derived(Data)]
pub enum EventsAnswer {
    Event(EventFrame),
    Pong(Ping),
    Close(StreamClose),
}

/// What a Status stream's client sends after the hello.
#[cereal::derived(Data)]
pub enum StatusFrame {
    Lookup(StatusQuery),
    Health { correlation: u64 },
    Ping(Ping),
    Close(StreamClose),
}

/// What the worker answers on a Status stream.
#[cereal::derived(Data)]
pub enum StatusAnswer {
    Lookup(StatusReply),
    Health { correlation: u64, observation: Observation },
    Pong(Ping),
    Close(StreamClose),
}
