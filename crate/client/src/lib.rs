pub(crate) mod session;

pub(crate) use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub(crate) use futures_util::{SinkExt, StreamExt};

pub(crate) use sourcetrait_grammar_protocol::{
    ClientHello, DecodeBudget, Emergency, EngineRequest, EngineResponse, FrameCodec, HELLO_BUDGET,
    HelloReply, HostAnswer, McpFrame, PING_INTERVAL, Ping, REPLY_DEADLINE, SESSION_IDLE,
    SessionAsk, SessionClose, TransportError, WorkerFrame, WorkerHello, check_reply,
    decode_payload, payload,
};

pub(crate) mod tk {
    pub(crate) use tokio::{
        io::{AsyncRead, AsyncWrite},
        spawn,
        sync::{
            mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
            oneshot,
        },
        task::JoinHandle,
        time::{sleep, timeout},
    };
}

pub(crate) mod tku {
    pub(crate) use tokio_util::bytes::Bytes;
    pub(crate) use tokio_util::codec::{FramedRead, FramedWrite};
}

pub use crate::session::{Responder, Session, SessionEvent};
