# listener.rs

## const SOCKET_LOCK_EXTENSION
The socket's own lock, `<key>.lock` beside `<key>.sock`. The namespace lock
lives in the data home and the socket in the runtime home, each set apart, so
two instances of one id and namespace with different data homes take different
namespace locks and still name one socket; the socket's lock is what keeps the
second from it.

## struct EngineListener
The worker's own socket, for a foreground or embedded start; a service's
comes from systemd's named descriptor once the service consumes it.

### fn bind
The directory is the boundary: made 0700 and refused unless a plain directory
the UID owns and opens to no one else, so no other user reaches the socket
whatever its own mode. The socket's lock is taken next, and refused while
another worker holds it. A socket already at the path is probed (`probe`): one
that answers is live, a listener this lock doesn't know, and is refused; one
that refuses the connect is a dead worker's and is removed. Anything else at
the path is refused, never replaced. The socket is set 0600 after the bind,
and a failure after the bind takes the socket with it.

## fn probe
A connect that never waits, since the start holds both locks: on Linux a
blocking connect to a listener whose backlog is full waits until it accepts,
which a stuck listener never does. A full backlog answers `EAGAIN` there, and
is a live listener. macOS answers a full backlog with `ECONNREFUSED`, as it
answers a socket nothing listens on, so there the probe reads a full listener
as dead; any worker holds the socket's lock first, so only a listener that
lock doesn't know can be one.

### field bound
The bound socket's device and inode: the drop removes the path only while it is
still this socket, so a successor's socket at the same path is never unlinked.
The socket's lock is released after the socket goes.
