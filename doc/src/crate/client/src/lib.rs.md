# lib.rs

The native SSH client: russh and russh-sftp embedded, no system ssh or sftp
process, and no transport Unix socket. It takes the protocol and portable
utilities alone, never the engine, the operator, or the daemon.

## mod artifact, config, credential, platform, ssh, transport, trust
Homes, empty until their code arrives, each a directory of submodules:
`artifact` the SFTP transfers, `config` the endpoint profiles, `credential` the
key files and the agent, `platform` the frontend's scratch, `ssh` the session
and its jump chain, `transport` the streamlocal channel and the hello, and
`trust` the known hosts, the host certificates, and revocation.

## pub use
The client's capabilities.
