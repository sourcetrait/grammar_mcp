# common.rs

## struct NuapiCall
The call a host decl acts for, resolved at RUN time from the stack's
`$env.NONCE`, which every call template sets and every job a call spawns
inherits, so a long run's packets still land in its launching call's trace. The
log dir follows from the nonce alone (`call_log_dir`), and the origin with it.
The nonce is random enough that run, call and interact need no distinguishing
prefix.

NOT BOUND AT PARSE TIME, and that is the point. A decl carrying its call's log
dir served every call but one: the interact engine persists, and a module an
earlier interact call parsed keeps its binding to that call's decl, so a later
call's `grimm dbg` appended to the earlier call's trace - measured on the old
host, the second call's mark landed in the first call's file and the second
call's trace was never written.

A body that rewrites `$env.NONCE` redirects its own artifacts; the nonce is
checked base62 before it names a path, so it can name only a call's own dir.

### fn stderr_log
Opened to append, as `eval_in_process` opens the stderr log, so this writer and
an external's never overwrite each other.

### fn origin
Host-stamped from the nonce: a body can claim another origin only by rewriting
its own `$env.NONCE`, and only one naming a call whose log dir exists.

### fn append_debug
`to nuon` renders compactly - no pretty-printing newlines between fields - but it
does NOT escape a newline INSIDE a string value; it emits the byte raw. Verified
rather than assumed: `{msg: "a\nb"}` comes back out of `to nuon` spanning two
lines, which breaks the one-record-per-line invariant a nuonl reader depends on.
So raw newline and carriage-return bytes are re-escaped here.

That substitution is safe precisely BECAUSE `data` is a record or table and the
render is compact: the only raw newlines the output can carry are inside
double-quoted strings, where `\n` and `\r` ARE the escapes nushell reads back, so
the line still parses to the original value. Backslashes `to nuon` already escaped
are untouched, since only the newline bytes themselves are replaced.

This is the shape of assumption that needs a live probe rather than a read of the
nushell source - the render's behaviour is not what the source suggests.

## fn require_record_or_table
The signature's `SyntaxShape` rejects a bad LITERAL at parse time, but a dynamic
argument (`grimm dbg $x`) reaches `run` unchecked - records are open and a list is
not shape-checked per element - so the guard has to live at run time too. One check
is genuinely not enough.

An empty list PASSES as a table, which matches the schema system's rule that `[]`
satisfies any table type.

## fn data_shape
Both members are column-OPEN: an empty `CollectionColumns` declares no required
columns, so any record or table binds.

## fn event_id_record
One record for the three sends, so `grimm channel send` and the two remote
sends cannot drift from the `{event_id}` their documented examples give. A
`let` is where nushell enforces a declared output type, so a decl returning
other than it declares fails there, as a bare `Type mismatch`: a string under
a record signature did exactly that.

## fn event_id_type
The declared half of the same contract, shared by the three signatures.

## fn register_nuapi
MUST run before `nu::parse`. The parser resolves command names at parse time, so a
decl added afterwards is invisible to the body that needed it.

THIS ONE SITE IS THE ACCESS CONTROL. It is called from exactly one place, inside
`eval_in_process`, so the decls are absent from every engine that is not
mid-eval - not on the stateless base, not on the interact base, and not in the
validator's `ParseEngine`. Off-host nushell cannot resolve the names at all. That
is a stronger trap than a signing scheme, because there is no reachable entry
point to authenticate. A committed rig CAN call them,
but only because its call-target body runs inside an eval.

Every decl here is stateless: `dbg` and `send` find their call at run time, and
the config trio reads process-global config and the pin layer. They register
here anyway rather than on the base, because this site is what keeps the whole
family unreachable outside an eval; the registration is the access control, not
the routing.

A CLEAN COMMIT PROVES NOTHING ABOUT RESOLUTION: `grimm ...` has its own command
head, so the validator parses it as an implicit external whether or not the decl
exists. Only an actual call is evidence that a committed body can reach the API.
