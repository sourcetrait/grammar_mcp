---
name: grammar-mcp
description: Harness shell tools powered by Nushell
---
# Grammar MCP
What the tool and server schemas leave out. The Nu language is in the nu
skill; equip authoring is in grammar-mcp-dev.

## In a definition
- `$env.NONCE`: this call's nonce. `$env.EQUIP_ID`, `EQUIP_NAMESPACE`, and
  `EQUIP_WORK_DIR`: the host's. `$env.EQUIP_TMP`: its bulk scratch directory;
  `$env.EQUIP_TRANSIENT`: its directory for small runtime files.
- Import a lib's or a rig's module by its path, never an equip's root:
  `use equip/<author>/<name>/lib/<module>`, then `<module> <export> ...`.
- A check reports at most three errors at a time; fix them and resubmit.
- `thread::dispatch`: the eval pool was busy; retry.

## Bulk
A result carries a summary. Bulk goes to a file under `$env.EQUIP_TMP`, its
path relative to it in a field named `*_tmp`. The directory is the host's
own, gone when the host ends; `interface info`'s `tmp` names it.

## grimm
The commands a definition reaches the host with:
- `grimm dbg <record|table>`: append to this call's `debug.nuonl`.
- `grimm channel send <model> <event> [attached]`: a packet on your Channel,
  `attached` kept in its inbox; answers `{event_id}`.
- `grimm remote channel send <nom> <model> <event>`: the same to a linked
  host; `grimm remote_channel_send_with` adds a `table<src, dest>` of files.
- `grimm get_config <key>` and `grimm get_config_all`: the config in force.
- `grimm pin_config <key> <pid> <value>`: hold a `supervisor.*` line while
  the pid lives.

## info and help
`interface info` indents one space a level: author, rig, modules, calls. A
call's line is `<name> <data> <result> # <summary>`:
```txt
acme
 geo # planar geometry helpers
  shape
   area <width:float,height:float> <area:float> # the inputs' unit squared
```
That is `call` on `acme/geo:shape:area` with `{width: 3.0, height: 4.0}`. A
builtin, `gram` or `grammar`, sits at an author's depth and is called by its
bare name: `gram:get:log`. `interface help` takes a namepath or a pattern:
`author/`, `author/rig:`, `.../mod/` its subtree, `.../mod:` its calls, `*`
everything, `.` the purview.

## Purviews
Named namepath patterns filtering what `info` shows, never access control:
`default` a fresh session's view, `core` the builtins, `.` the current view,
`*` everything, `@id` another purview's patterns.

## Channel
Packets from a definition to you: small summaries, never payloads.
- async: `interface channel open async`, a Monitor on its `wss`, the
  `mcp/channel/Open` packet, then `interface channel ready`.
- sync: `interface channel open sync`; the first `interface channel sync`
  takes the greeting, then `interface channel ready`; each sync after takes
  what queued since.
- A send before `ready` is refused and closes the Channel. `existing` is
  normal: keep the Monitor. On `channel::peer_gone`, close and open again.
- A packet is `{id, from, model, event, attached?}`; models under `mcp/` are
  the host's. An oversized event spills to `<inbox>/<spilled_event_path>`.
- Sending too fast warns, then stops the sender.

## RemoteChannel
Links two hosts' Channels. `.grammar/config.toml`, in the directory the host
starts in, names its key pair as `[identity]` and each peer as a `[[remote]]`
alias; peers trust each other by known public key.
1. Both Channels open and ready.
2. `interface remote open <alias>`: the listener first, then the connector.
3. Send to the peer's nom from `interface remote list`, never the alias.
4. `mcp/remote/Sent` or `Unsent` reports a send; a reply comes from
   `mcp/remote/<nom>`.
