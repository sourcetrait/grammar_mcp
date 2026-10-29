# daemon fixtures

Public fixtures for the Grammar daemon's qualification. Nothing here is
private: no key's private half, no account, no host, and no path.

- `identity_manifest.schema.json`: the JSON Schema of a fixture run's
  endpoint descriptor. The private copy names its credential files; the
  redacted copy omits them; neither holds a key's body.
- `trust_vectors.json`: SSH trust and authentication vectors under the
  placeholder `endpoint.invalid`. Test-only keys, a host CA and an untrusted
  one, host certificates valid, expired, not yet valid, for the wrong
  principal, revoked by a KRL, and from the untrusted CA, and known_hosts
  variants, each case with the class OpenSSH's own client observed: accept,
  host_trust, auth, or forward. The testbed's fixture tool generated them
  against an sshd on loopback presenting each certificate in turn; their
  private keys stayed with that run. `openssh` names the client.

`tests/daemon_fixtures.rs` holds both files to these rules.
