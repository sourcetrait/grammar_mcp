//! An admitted stream served by its role until it closes or its lease ends it.
use crate::*;

/// The reason a call is refused before Dispatch serves calls.
const CALLS_UNSERVED: &str = "this worker serves no calls before Dispatch";

/// A framed stream after its hello.
pub(crate) type Framed = tk_util::codec::Framed<tk::net::UnixStream, FrameCodec>;

/// What a role's loop does with one frame: go on, answer, or close.
enum Turn<A> {
    Go,
    Answer(A),
    Close(Option<TransportError>),
}

/// `framed` served under `accepted` until either side ends it, then deregistered.
pub(crate) async fn serve(
    mut framed: Framed,
    sessions: Arc<Sessions>,
    accepted: ServerHello,
    id: StreamId,
    stop: StopRing,
) {
    framed.codec_mut().set_ceiling(accepted.effective.frame_ceiling(accepted.role));
    let budget = accepted.effective.decode_budget();
    match accepted.role {
        StreamRole::Rpc => run(&mut framed, stop, budget, |frame| rpc_turn(&sessions, frame), RpcAnswer::Close).await,
        StreamRole::Events => run(&mut framed, stop, budget, events_turn, EventsAnswer::Close).await,
        StreamRole::Status => run(&mut framed, stop, budget, status_turn, StatusAnswer::Close).await,
    }
    sessions.deregister(id);
}

/// One role's loop: each frame decoded within `budget` and turned, until a close.
async fn run<F, A>(
    framed: &mut Framed,
    mut stop: StopRing,
    budget: DecodeBudget,
    mut turn: impl FnMut(F) -> Turn<A>,
    close: fn(StreamClose) -> A,
) where
    F: bitcode::DecodeOwned,
    A: bitcode::Encode,
{
    loop {
        let next = tk::select! {
            frame = framed.next() => match frame {
                Some(Ok(bytes)) => match decode_payload::<F>(&bytes, budget) {
                    Ok(frame) => turn(frame),
                    Err(e) => Turn::Close(Some(e.transport())),
                },
                _ => return,
            },
            changed = stop.changed() => match changed {
                Ok(()) => Turn::Close(stop.borrow().clone()),
                Err(_) => return,
            },
        };
        match next {
            Turn::Go => {}
            Turn::Answer(answer) => {
                if framed.send(payload(&answer)).await.is_err() {
                    return;
                }
            }
            Turn::Close(error) => {
                let _ = framed.send(payload(&close(StreamClose { error }))).await;
                return;
            }
        }
    }
}

/// An RPC frame: a lease message answered, a ping ponged, a call refused.
fn rpc_turn(
    sessions: &Sessions,
    frame: RpcFrame,
) -> Turn<RpcAnswer> {
    match frame {
        RpcFrame::Lease(request) => Turn::Answer(RpcAnswer::Lease(sessions.request(&request))),
        RpcFrame::Ping(ping) => Turn::Answer(RpcAnswer::Pong(ping)),
        RpcFrame::Close(_) => Turn::Close(None),
        RpcFrame::Ticket(_) | RpcFrame::Submit(_) | RpcFrame::Status(_) => Turn::Close(Some(unserved())),
    }
}

/// An Events frame: a ping ponged, a subscription's frames held for EventBroker.
fn events_turn(frame: EventsFrame) -> Turn<EventsAnswer> {
    match frame {
        EventsFrame::Ping(ping) => Turn::Answer(EventsAnswer::Pong(ping)),
        EventsFrame::Close(_) => Turn::Close(None),
        EventsFrame::Resume(_) | EventsFrame::Ack(_) => Turn::Go,
    }
}

/// A Status frame: a ping ponged, a lookup or a health check refused for now.
fn status_turn(frame: StatusFrame) -> Turn<StatusAnswer> {
    match frame {
        StatusFrame::Ping(ping) => Turn::Answer(StatusAnswer::Pong(ping)),
        StatusFrame::Close(_) => Turn::Close(None),
        StatusFrame::Lookup(_) | StatusFrame::Health { .. } => Turn::Close(Some(unserved())),
    }
}

/// The refusal of a call no task here serves yet.
fn unserved() -> TransportError {
    TransportError::Unavailable {
        reason: CALLS_UNSERVED.to_string(),
    }
}
