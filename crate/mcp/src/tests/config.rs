use crate::*;
use crate::config::ConfigToml;

/// Build a runtime config from file text alone; the argument-owned values are
/// placeholders, since the file surface is what these lock.
fn from_text(text: &str) -> Result<Config, String> {
    let toml: ConfigToml = toml::from_str(text).map_err(|e| e.to_string())?;
    Config::from_toml(
        toml,
        "id".to_string(),
        "ns".to_string(),
        PathBuf::from("/w"),
        DenySet::default(),
        false,
    )
}

/// An `[identity]` table naming a key pair under `/keys`.
const IDENTITY: &str = "\
[identity]
cert_file = \"/keys/self.pem\"
key_file = \"/keys/self.key.pem\"
";

#[test]
fn an_empty_file_yields_the_embedded_defaults() {
    let config = from_text("").expect("empty file is valid");
    assert_eq!(
        config.channel.port, None,
        "an unpinned port must stay None, not become a 0 sentinel",
    );
    assert!(config.identity.is_none(), "the embedded base names no identity");
    assert!(config.remotes.by_alias.is_empty(), "and no remote");
}

#[test]
fn a_privileged_or_zero_port_is_rejected() {
    // 0 is the syscall's "assign me one" convention, never a configurable value; below
    // 1024 needs root, which the host never has, so it could only fail at bind.
    for port in [0, 80, 443, 1023] {
        let err = from_text(&format!("[channel]\nport = {port}\n"))
            .expect_err(&format!("port {port} must be rejected"));
        assert!(err.contains("1024-65535"), "port {port}: got {err}");
    }
}

#[test]
fn the_lowest_unprivileged_port_is_accepted() {
    let config = from_text("[channel]\nport = 1024\n").expect("1024 is valid");
    assert_eq!(config.channel.port, Some(1024));
}

#[test]
fn the_namespace_cannot_be_set_from_a_file() {
    // The whole point of deny_unknown_fields here: a file must not be able to move the
    // namespace out from under the .mcp.json entry that launched the server.
    for text in [
        "id = \"other\"\n",
        "namespace = \"other\"\n",
        "work_dir = \"/elsewhere\"\n",
        "deny = [\"run\"]\n",
        // `--test` joins the argument-owned set for the same reason: it selects
        // the namespace, so a file-settable one could move the namespace out from
        // under the .mcp.json entry that launched the server.
        "test = true\n",
    ] {
        assert!(
            from_text(text).is_err(),
            "{text:?} must be rejected as an unknown field",
        );
    }
}

#[test]
fn the_test_flag_is_carried_on_the_runtime_config() {
    let toml: ConfigToml = toml::from_str("").expect("empty file is valid");
    let config = Config::from_toml(
        toml,
        "id".to_string(),
        TEST_NAMESPACE.to_string(),
        PathBuf::from("/w"),
        DenySet::default(),
        true,
    )
    .expect("valid");
    assert!(config.test, "the watchdog half reads this at every tick");
    assert!(
        !from_text("").expect("valid").test,
        "and an ordinary host is not a test host",
    );
}

#[test]
fn a_retired_channel_key_is_rejected() {
    // bind / cert_name / verify_timeout_secs / cert_dir were removed; a stale file must
    // fail loudly rather than be silently ignored.
    for text in [
        "[channel]\nbind = \"0.0.0.0\"\n",
        "[channel]\ncert_name = \"custom\"\n",
        "[channel]\nverify_timeout_secs = 42\n",
        "[channel]\ncert_dir = \"/somewhere/certs\"\n",
    ] {
        assert!(from_text(text).is_err(), "{text:?} must be rejected");
    }
}

#[test]
fn an_identity_names_both_files_expanded() {
    let config =
        from_text("[identity]\ncert_file = \"~/k/self.pem\"\nkey_file = \"~/k/self.key.pem\"\n")
            .expect("valid");
    let identity = config.identity.expect("an identity");
    assert!(
        identity.cert_file.is_absolute() && identity.key_file.is_absolute(),
        "a `~` path must be expanded at load; got {} and {}",
        identity.cert_file.display(),
        identity.key_file.display(),
    );
    assert!(identity.cert_file.ends_with("k/self.pem"));
    assert!(identity.key_file.ends_with("k/self.key.pem"));
}

#[test]
fn an_identity_missing_a_file_is_rejected() {
    for (text, field) in [
        ("[identity]\ncert_file = \"/k/self.pem\"\n", "key_file"),
        ("[identity]\nkey_file = \"/k/self.key.pem\"\n", "cert_file"),
    ] {
        let err = from_text(text).expect_err("an identity names both files");
        assert!(err.contains(field), "{text:?}: got {err}");
    }
}

#[test]
fn an_unset_variable_fails_the_load() {
    // Expansion happens at the boundary, so an unresolvable path is a config error
    // rather than something that surfaces much later at first use. Only a home, a
    // UENV one or a standard one, falls back; any other unset variable is
    // unresolvable.
    let err = from_text(
        "[identity]\ncert_file = \"$GRAMMAR_TEST_UNSET_VAR/self.pem\"\nkey_file = \"/k\"\n",
    )
    .expect_err("an unset variable outside the XDG homes must fail the load");
    assert!(err.contains("GRAMMAR_TEST_UNSET_VAR"), "got {err}");
}

#[test]
fn the_xdg_homes_carry_spec_defaults() {
    // The expansion context falls back to these when a variable is unset or empty;
    // every other variable has none and stays a load error.
    let home = PathBuf::from(std::env::var("HOME").expect("HOME set"));
    for (var, home_relative) in [
        ("XDG_CACHE_HOME", ".cache"),
        ("XDG_CONFIG_HOME", ".config"),
        ("XDG_DATA_HOME", ".local/share"),
        ("XDG_STATE_HOME", ".local/state"),
    ] {
        assert_eq!(
            crate::config::spec_default(var),
            Some(home.join(home_relative)),
            "{var} must default under $HOME as the XDG spec has it",
        );
    }
    for var in ["XDG_RUNTIME_DIR", "GRAMMAR_TEST_UNSET_VAR"] {
        assert_eq!(
            crate::config::spec_default(var),
            None,
            "{var} has no default and stays a load error",
        );
    }
}

#[test]
fn a_uenv_home_stands_in_front_of_its_standard_variable() {
    for (uenv, standard) in [
        ("UENV_USR_CACHE", "XDG_CACHE_HOME"),
        ("UENV_USR_CONFIG", "XDG_CONFIG_HOME"),
        ("UENV_USR_DATA", "XDG_DATA_HOME"),
        ("UENV_USR_STATE", "XDG_STATE_HOME"),
        ("UENV_USR_TMP", "TMPDIR"),
        ("UENV_USR_TRANSIENT", "XDG_RUNTIME_DIR"),
    ] {
        assert_eq!(
            crate::home::uenv_standard(uenv),
            Some(standard),
            "{uenv} stands in front of {standard}",
        );
        assert_eq!(crate::home::uenv_partner(standard), Some(uenv), "and {standard} behind {uenv}");
    }
    assert_eq!(
        crate::home::uenv_standard("UENV_USR_SECRET"),
        None,
        "a UENV tree with no standard variable stands alone",
    );
    assert_eq!(
        crate::config::spec_default("TMPDIR"),
        Some(PathBuf::from("/tmp")),
        "TMPDIR defaults to /tmp",
    );
}

#[test]
fn the_agent_file_is_read_from_its_directory() {
    let dir = tempfile::tempdir().expect("a directory");
    let none = Config::read_agent_toml(dir.path()).expect("an absent file is none");
    assert!(none.identity.is_none() && none.remote.is_empty());
    let grammar = dir.path().join(".grammar");
    fs::create_dir_all(&grammar).expect("its .grammar");
    fs::write(grammar.join("config.toml"), "[channel]\nport = 47800\n").expect("its file");
    let file = Config::read_agent_toml(dir.path()).expect("the file is read");
    assert_eq!(file.channel.and_then(|channel| channel.port), Some(47800));
}

#[test]
fn a_remote_entry_with_listen_is_a_listener() {
    let text = format!(
        "{IDENTITY}
[[remote]]
alias = \"bob\"
listen = \"127.0.0.1:9000\"
address = \"127.0.0.1\"
public_key_file = \"/keys/bob.pem\"
"
    );
    let config = from_text(&text).expect("valid listener entry");
    let entry = config.remotes.by_alias.get("bob").expect("bob present");
    match &entry.role {
        RemoteRole::Listener { bind, allow } => {
            assert_eq!(bind.to_string(), "127.0.0.1:9000");
            // `address` on a listener is a bare source-IP filter.
            assert_eq!(allow.map(|ip| ip.to_string()), Some("127.0.0.1".to_string()));
        }
        other => panic!("expected a listener, got {other:?}"),
    }
    assert!(entry.peer_public_key_file.ends_with("bob.pem"));
    assert!(
        entry.self_public_key_file.ends_with("self.pem")
            && entry.self_private_key_file.ends_with("self.key.pem"),
        "a link presents the host's [identity]",
    );
}

#[test]
fn a_listener_address_with_a_port_is_rejected() {
    // A listener's `address` is a bare source-IP filter; the host:port form is the
    // connector's dial-target format and must not be accepted here.
    let text = format!(
        "{IDENTITY}
[[remote]]
alias = \"bob\"
listen = \"127.0.0.1:9000\"
address = \"127.0.0.1:5555\"
public_key_file = \"/keys/bob.pem\"
"
    );
    let err = from_text(&text).expect_err("a host:port listener address must fail");
    assert!(err.contains("bare IP"), "got {err}");
}

#[test]
fn a_remote_entry_without_listen_is_a_connector() {
    let text = format!(
        "{IDENTITY}
[[remote]]
alias = \"bob\"
address = \"127.0.0.1:9001\"
public_key_file = \"/keys/bob.pem\"
"
    );
    let config = from_text(&text).expect("valid connector entry");
    match &config.remotes.by_alias.get("bob").expect("bob present").role {
        RemoteRole::Connector { addr } => {
            assert_eq!(addr.to_string(), "127.0.0.1:9001");
        }
        other => panic!("expected a connector, got {other:?}"),
    }
}

#[test]
fn a_connector_without_an_address_is_rejected() {
    let text = format!(
        "{IDENTITY}
[[remote]]
alias = \"bob\"
public_key_file = \"/keys/bob.pem\"
"
    );
    let err = from_text(&text).expect_err("a connector needs an address");
    assert!(err.contains("address"), "got {err}");
}

#[test]
fn a_remote_entry_missing_its_peer_key_is_rejected() {
    let text = format!("{IDENTITY}\n[[remote]]\nalias = \"bob\"\naddress = \"127.0.0.1:9001\"\n");
    let err = from_text(&text).expect_err("a missing peer key must fail");
    assert!(err.contains("bob") && err.contains("public_key_file"), "got {err}");
}

#[test]
fn a_remote_without_an_identity_is_rejected() {
    let text = "\
[[remote]]
alias = \"bob\"
address = \"127.0.0.1:9001\"
public_key_file = \"/keys/bob.pem\"
";
    let err = from_text(text).expect_err("a link presents the host's identity");
    assert!(err.contains("[identity]"), "got {err}");
}

#[test]
fn a_retired_remote_key_is_rejected() {
    // A link's own key pair is the host's [identity]; a stale per-link pair must fail
    // loudly rather than be silently ignored.
    let text = format!(
        "{IDENTITY}
[[remote]]
alias = \"bob\"
address = \"127.0.0.1:9001\"
self_public_key_file = \"/keys/self.pem\"
public_key_file = \"/keys/bob.pem\"
"
    );
    assert!(from_text(&text).is_err(), "a per-link self key must be rejected");
}

#[test]
fn a_typoed_remote_field_is_rejected() {
    // deny_unknown_fields on the entry catches a field typo loudly.
    let text = format!("{IDENTITY}\n[[remote]]\nalias = \"bob\"\naddres = \"127.0.0.1:9001\"\n");
    assert!(from_text(&text).is_err(), "a typo'd field must be rejected");
}

#[test]
fn a_duplicate_alias_is_rejected() {
    let entry = "[[remote]]\nalias = \"bob\"\naddress = \"127.0.0.1:9001\"\npublic_key_file = \"/keys/bob.pem\"\n";
    let err = from_text(&format!("{IDENTITY}\n{entry}{entry}"))
        .expect_err("an alias names one peer");
    assert!(err.contains("duplicate"), "got {err}");
}

#[test]
fn a_bad_remote_address_fails_the_load() {
    let text = format!(
        "{IDENTITY}
[[remote]]
alias = \"bob\"
address = \"not-an-address\"
public_key_file = \"/keys/bob.pem\"
"
    );
    let err = from_text(&text).expect_err("a non ip:port address must fail");
    assert!(err.contains("is not ip:port"), "got {err}");
}
