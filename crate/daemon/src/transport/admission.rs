//! Admission: a stream's peer held to the worker's UID before its hello is read.
use crate::*;

/// The pause after a failed accept, so a full descriptor table doesn't spin.
const ACCEPT_BACKOFF: std::time::Duration = std::time::Duration::from_millis(100);

/// What every stream is admitted by: the worker's UID, profile, lease, and budget.
#[derive(Debug)]
pub(crate) struct Admission {
    profile: WorkerProfile,
    leases: std::sync::Mutex<LeaseTable>,
    streams: Arc<tk::sync::Semaphore>,
    epoch: std::time::Instant,
}

impl Admission {
    /// Admission for `profile`'s worker, its streams at once held to its limits.
    pub(crate) fn new(profile: WorkerProfile) -> Self {
        Self {
            leases: std::sync::Mutex::new(LeaseTable::new(profile.generation)),
            streams: Arc::new(tk::sync::Semaphore::new(profile.limits.streams as usize)),
            epoch: std::time::Instant::now(),
            profile,
        }
    }

    /// The reply to `hello`, its lease bound in the table; none without fresh ids.
    fn answer(
        &self,
        hello: &ClientHello,
    ) -> Option<HelloReply> {
        let drawn: [u8; 48] = random_bytes().ok()?;
        let (lease, token) = drawn.split_at(16);
        let lease = LeaseId(lease.try_into().ok()?);
        let token = LeaseToken(token.try_into().ok()?);
        let now_ms = u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut leases = self.leases.lock().unwrap_or_else(|e| e.into_inner());
        let mut state = HelloState::default();
        let (reply, _actions) = state.on_hello(hello, &self.profile, &mut leases, now_ms, Fresh { lease, token });
        Some(reply)
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

impl Admission {
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

/// One admitted stream: its hello answered within the deadline, its place held.
async fn admit(
    stream: tk::net::UnixStream,
    admission: Arc<Admission>,
    _held: tk::sync::OwnedSemaphorePermit,
) {
    let mut framed = framed(stream);
    let Ok(Some(Ok(frame))) = tk::time::timeout(HELLO_DEADLINE, framed.next()).await else {
        return;
    };
    let reply = match decode_payload::<ClientHello>(&frame, HELLO_BUDGET) {
        Ok(hello) => match admission.answer(&hello) {
            Some(reply) => reply,
            None => return,
        },
        Err(e) => HelloReply::Refused(e.transport()),
    };
    let _ = tk::time::timeout(HELLO_DEADLINE, framed.send(payload(&reply))).await;
}
