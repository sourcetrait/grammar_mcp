//! Admission: a stream's peer held to the worker's UID before its hello is read.
use crate::*;

/// The pause after a failed accept, so a full descriptor table doesn't spin.
const ACCEPT_BACKOFF: std::time::Duration = std::time::Duration::from_millis(100);

/// What every stream is admitted by: the worker's profile, sessions, and budget.
#[derive(Debug)]
pub(crate) struct Admission {
    profile: WorkerProfile,
    sessions: Arc<Sessions>,
    streams: Arc<tk::sync::Semaphore>,
}

impl Admission {
    /// Admission for `profile`'s worker over `sessions`, its streams within limits.
    pub(crate) fn new(
        profile: WorkerProfile,
        sessions: Arc<Sessions>,
    ) -> Self {
        Self {
            streams: Arc::new(tk::sync::Semaphore::new(profile.limits.streams as usize)),
            profile,
            sessions,
        }
    }

    /// The reply to `hello`, its stream registered if bound; none without fresh ids.
    fn answer(
        &self,
        hello: &ClientHello,
    ) -> Option<(HelloReply, Option<(StreamId, StopRing)>)> {
        let drawn: [u8; 48] = random_bytes().ok()?;
        let (lease, token) = drawn.split_at(16);
        let fresh = Fresh {
            lease: LeaseId(lease.try_into().ok()?),
            token: LeaseToken(token.try_into().ok()?),
        };
        Some(self.sessions.admit(hello, &self.profile, fresh))
    }

    /// A place in the budget for `stream`, from the worker's own UID; none closes it.
    fn hold(
        &self,
        stream: &tk::net::UnixStream,
    ) -> Option<tk::sync::OwnedSemaphorePermit> {
        let held = self.streams.clone().try_acquire_owned().ok()?;
        match stream.peer_cred() {
            Ok(cred) if cred.uid() == self.profile.uid => Some(held),
            Ok(cred) => {
                eprintln!("grammard: a stream from UID {} is closed unread", cred.uid());
                None
            }
            Err(e) => {
                eprintln!("grammard: a stream's peer can't be read, so it is closed unread: {e}");
                None
            }
        }
    }
}

/// Streams accepted on `listener` until `stop` changes, each admitted on a task.
pub(crate) async fn accept(
    listener: tk::net::UnixListener,
    admission: Arc<Admission>,
    mut stop: tk::sync::watch::Receiver<bool>,
) {
    loop {
        tk::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    if let Some(held) = admission.hold(&stream) {
                        tk::spawn(admit(stream, admission.clone(), held));
                    }
                }
                Err(e) => {
                    eprintln!("grammard: an accept failed: {e}");
                    tk::time::sleep(ACCEPT_BACKOFF).await;
                }
            },
            _ = stop.changed() => break,
        }
    }
}

/// One admitted stream: its hello answered within the deadline, then served.
async fn admit(
    stream: tk::net::UnixStream,
    admission: Arc<Admission>,
    _held: tk::sync::OwnedSemaphorePermit,
) {
    let mut framed = framed(stream);
    let Ok(Some(Ok(frame))) = tk::time::timeout(HELLO_DEADLINE, framed.next()).await else {
        return;
    };
    let (reply, registered) = match decode_payload::<ClientHello>(&frame, HELLO_BUDGET) {
        Ok(hello) => match admission.answer(&hello) {
            Some(answered) => answered,
            None => return,
        },
        Err(e) => (HelloReply::Refused(e.transport()), None),
    };
    let accepted = match &reply {
        HelloReply::Accepted(accepted) => Some(accepted.as_ref().clone()),
        HelloReply::Refused(_) => None,
    };
    let sent = tk::time::timeout(HELLO_DEADLINE, framed.send(payload(&reply))).await;
    match (registered, accepted) {
        (Some((id, ring)), Some(accepted)) if matches!(sent, Ok(Ok(()))) => {
            serve(framed, admission.sessions.clone(), accepted, id, ring).await;
        }
        (Some((id, _)), _) => admission.sessions.deregister(id),
        _ => {}
    }
}
