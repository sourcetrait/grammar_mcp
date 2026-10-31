# listener.rs

## struct EngineListener
The worker's own socket, for a foreground or embedded start; a service's
comes from systemd's named descriptor once the service consumes it.

### fn bind
The directory is the boundary: made 0700 and refused unless a plain directory
the UID owns and opens to no one else, so no other user reaches the socket
whatever its own mode. A socket left at the path is a dead worker's, since the
namespace lock this worker holds admits no other of its key, and is removed;
anything else there is refused, never replaced. The socket is set 0600 after
the bind, and a failure after the bind takes the socket with it.

### field bound
The bound socket's device and inode: the drop removes the path only while it is
still this socket, so a successor's socket at the same path is never unlinked.
