//! An embedding host's use of the runtime library, without MCP or the service.

use sourcetrait_grammar_daemon::{DaemonError, Runtime, RuntimeIdentity};
use sourcetrait_grammar_protocol::Capabilities;

#[test]
fn a_process_starts_one_runtime() {
    let identity = RuntimeIdentity { uid: 1000, id: "embedder".into(), namespace: "test".into() };
    let runtime = Runtime::start(identity.clone()).expect("the first start");
    assert_eq!(runtime.identity(), &identity);
    assert_eq!(runtime.capabilities(), Capabilities::NONE);

    let other = RuntimeIdentity { namespace: "other".into(), ..identity.clone() };
    assert_eq!(Runtime::start(other).err(), Some(DaemonError::AlreadyStarted));

    runtime.shutdown().expect("the shutdown");
    assert_eq!(
        Runtime::start(identity).err(),
        Some(DaemonError::AlreadyStarted),
        "a process starts no second runtime, after a shutdown too",
    );
}
