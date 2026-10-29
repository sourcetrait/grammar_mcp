# rerun.rs

## const RERUN_DESCRIPTION
The description in one place, so the client and the test driver advertise the
same words (`server/client.rs`).

## fn rerun
`execute` again on an earlier call's definition with fresh data: the same parameters as `execute`
less the definition, which the nonce names.

A FRESH nonce is minted for the rerun and the earlier one rides in the request as its
source; the host is resolved next (`resolve_host`, `common.rs`); the engine gates the
source, reads its cached definition, and runs it under the fresh nonce (the engine's
`serve.rs`).
