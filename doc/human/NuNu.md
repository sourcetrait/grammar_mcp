> REIN HUMAN

Tools go under their subheaders with tool parameters directly beneath the header.

The rest of the content for each section is intended to be str included,
human rein file, as the literal tool schema documentation.

# MCP

Grammar MCP tool calls primarily use parameters and results in the text form of:
- Nu
- NUON
- Namepath (`<author>/<name>:<module/path>:<function>`)
- Nonce (base62 pseudo-random xxh3-64 hash)

On success, the MCP JSON-RPC `result.content` will be populated with NUON text,
typed as:
```nu
record<
	nonce: string
	result: oneof<nothing, record<TYPEDEF>, table<TYPEDEF>>
>
```

On failure, the MCP JSON-RPC `result.content` will be populated with NUON text,
typed as:
```nu
record<
	nonce: string
	error: table<
		kind: string
		source: oneof<nothing, record<
			path: oneof<nothing, string>
			position: list<int>
		>>
		message: string
	>
>
```

# TOOL: grammar/nu
  definition: string
  data: string

Executes the provided Nu language definition against the NUON `$data` provided.

## Language Contract
This contract is aggressively enforced and will error if broken.
### General
- One top-level definition named `execute` that uses a Nu infix type signature
- Only allowed parameters: `data` and `timeout`
  - `data` parameter is mandatory
- The infix `input` type must always be `nothing`
- Values must be passed as data (see Lint)
### `timeout` Type
- Must be either optional (`timeout?`) or omitted from signature
- Value is inspected internally for tool timeouts; Default: `120sec`
### `data` Type
- Typed as:
  - `nothing`
  - `record<TYPEDEF>` with a strict type definition
  - `table<TYPEDEF>` with a strict type definition
- Denies untyped `record<>` or `table<>` at its top level
  - Nested fields allow this
- Denies undeclared fields
  - Nested untyped `record<>` and `table<>` allow this
- Denies lists of tables
### `return` Type
- Infix return type typed as:
  - `record<TYPEDEF>` with a strict type definition
  - `table<TYPEDEF>` with a strict type definition
- Denies untyped `record<>` or `table<>`
- Denies undeclared fields
- Denies lists of tables
### Lint
- Denies hard-coded paths starting with: `/ ~/ ./ ../`
  - Allows URLs
  - Allows well-known system paths starting with: `/dev/ /etc/ /proc/ /sys/`
	- Allows use with: `use` `overlay use` `source` `source-env`
- Denies externals (`^<cmd>`) handled better with Nushell:
    awk bash cp date echo find grep jq ls nu rg rm sed sh sort which

## Operation

Execution can be either stateless (default) or stateful (`--env`).

Stateless operation runs across a pool of isolated stateless workers.

Stateful execution is queued against a single stateful worker and persists
environment changes across the worker's long-term context.

## Examples
Note: Nonces would be random.
### Stateless execution
Tool parameter `definition`:
```nu
def execute [data: table<num: int>, timeout?: duration]: nothing -> table<id: int, name: string> {
	let total = $data | get num | math sum
	[
		[id name];
		[0 zero]
		[1 ($total | into string)]
	]
}
```
Tool parameter `data`:
```nuon
[[num]; [5], [3], [2]]
```
Tool result:
```nuon
{nonce: "5zNe1n9JO9G", result: [[id, name]; [0, zero], [1, "10"]]}
```
### Stateful execution
Tool parameter `definition`:
```nu
def --env execute [data: record<dir: directory, file: path>, timeout?: duration]: nothing -> record<id: int, name: string> {
	cd $data.dir
	let kind = open --raw $data.file | from yaml | get kind  # hypothetical: value retrieved is 'foobar'
	let count = ls ($data.dir | path join '*.md' | into glob) | length
	$count | save -f ($data.file + '.count')
	{
		id: 3
		name: $kind
	}
}
```
Tool parameter `data`:
```nuon
{dir: /tmp, file: barn.dat}
```
Tool result:
```nuon
{nonce: aOiwlfCpNKg, result: {id: 3, name: foobar}}
```

# TOOL: grammar/interface
 command: string

Interfaces directly with the Grammar MCP.

Runs a single MCP `interface` Nu Command in an isolated Nu context without
access to any other commands.

## Command Signatures
### General
- `def 'interface help' [namepath: string]: nothing -> record<>` Displays equipment documentation
- `def 'interface info' []: nothing -> record<>` MCP details, Equipment signatures within current purview
### Equip
- `def 'interface equip new' [namepaths: list<string>]: nothing -> record<created: list<string>>` Scaffold modules / functions (by namepath) into an equip's rig.
- `def 'interface equip commit' [namepath: string]: nothing -> record<added: list<string>, modified: list<string>, removed: list<string>>` Commit an equip's lib and rig from its source to the MCP's repository for live use.
- `def 'interface equip' [action: string, source_dir: string]: nothing -> record<summary: oneof<nothing, record<created: list<string>>, record<added: list<string>, modified: list<string>, removed: list<string>>, record<ok: bool, errors: table<kind: string, source: oneof<nothing, record<path: oneof<nothing, string>, position: list<int>>>, message: string>, warnings: table<kind: string, source: oneof<nothing, record<path: oneof<nothing, string>, position: list<int>>>, message: string>>>>` Equip administration on an equip/<author>/<name> source dir: new, install, check, uninstall. (`summary` per action: new, install, check; omitted for uninstall)
### Purview
### Channel
- `def 'interface channel open' []: nothing -> record<id: string>` Opens the Channel
- `def 'interface channel ready' []: nothing -> nothing` Verifies Channel activation and begins normal use.
- `def 'interface channel close' []: nothing -> nothing` Closes the channel

### General
- `def 'processes' []: nothing -> record<processes: table<nonce: string, tool: string, started_at: int, args: record, source_nonce: oneof<nothing, string>, path: oneof<nothing, string>>>` List in-flight MCP tool usage. (`started_at` epoch ms; `source_nonce` rerun only, `path` call only, each omitted when unset)
- `def 'kill' [nonce: string]: nothing -> nothing` Cancel an in-flight usage by its nonce.
- `def 'info' [purviews?: list<string>]: nothing -> record<name: string, version: string, nu_version: string, id: string, namespace: string, mcp_nom: string, work_dir: string, plugins: list<list<oneof<string, nothing>>>, signatures: string, purview: list<list<oneof<string, list<string>>>>>` Versions, plugins, and every rig's callable signatures. (`purviews` defaults to `[]`, the current view; `plugins` pairs `[name, version]`, `purview` pairs `[id, patterns]`)
- `def 'inspect' [namepath: string]: nothing -> record<doc: oneof<record<srcdir: string, summary: string, details: string>, record<src: string, summary: string, details: string>, record<src: string, signature: string, details: string>, record<signatures: string>>>` Detailed documentation for rigs, modules, and calls. (`doc` a rig's, a module's, a call's, or a pattern's signatures block)
### Equip
### Purview
- `def 'purviews' []: nothing -> record<purviews: list<list<oneof<string, list<string>>>>, current: list<string>>` List every configured purview, and what is currently in view. (`purviews` pairs `[id, patterns]`)
- `def 'purview_configure' [purview: string, namepaths: list<string>]: nothing -> record<signatures: oneof<nothing, string>>` Set a purview's namepath patterns; an empty list deletes it. (`signatures` null on a delete)
- `def 'purview_extend' [purviews: list<string>]: nothing -> record<revealed: oneof<nothing, string>, current: list<string>>` Bring more purviews into the current view.
- `def 'purview' [purviews: list<string>]: nothing -> record<revealed: oneof<nothing, string>>` Set the current view to these purviews.
### Channel
- `def 'channel_open' []: nothing -> record<status: string, wss: string, inbox: string>` Open the host's packet channel and return the endpoint to watch. (`status` new or existing)
- `def 'channel_verified' []: nothing -> nothing` Confirm the channel/Open packet was seen; ends the verify window.
- `def 'channel_close' []: nothing -> nothing` Close the host's packet channel.
- `def 'config_channel' [spam_warn_window_secs?: int, spam_warn_rate?: int, spam_error_window_secs?: int, spam_error_rate?: int]: nothing -> record<spam_warn_window_secs: int, spam_warn_rate: int, spam_error_window_secs: int, spam_error_rate: int>` Read or adjust the channel's send-rate thresholds at runtime.
### Remote
- `def 'remote_channel_open' [alias: string]: nothing -> nothing` Open a configured remote link by alias; blocks on bind (listener) or connect (connector, 20s) and returns synchronously - void on success, an error envelope on failure. A listener then emits mcp/remote/Connected when a peer pairs; Disconnected fires on any established-link teardown.
- `def 'remote_channel_close' [alias: string]: nothing -> nothing` Close an open mTLS link to a remote grammar host.
- `def 'remote_channels' []: nothing -> record<channels: table<alias: string, remote_mcp_nom: string, address: string>, listening: table<remote: string, address: string>>` List remote grammar-host links: established `channels` plus bound-but-unpaired `listening` listeners.
### Grimm
- `def 'grimm dbg' [data: oneof<record, table>]: nothing -> nothing` Append a record or table to this eval's debug.nuonl in its nonce log dir.
- `def 'grimm channel send' [model: string, event: oneof<record, table>, attached?: oneof<record, table>]: nothing -> record<event_id: string>` Emits a message to the Channel. Returns a message id on success.
- `def 'grimm remote channel send' [mcp_nom: string, model: string, event: oneof<record, table>]: nothing -> record<event_id: string>` Emits a message to a remote Channel. Returns a message id on success.
- `def 'grimm remote_channel_send_with' [mcp_nom: string, model: string, event: oneof<record, table>, attached: table]: nothing -> record<event_id: string>` Deliver a message plus files to a linked remote host's agent; returns the message id. (`attached` the files `table<src: path, dest: path>`)
- `def 'grimm get_config_all' []: nothing -> record` The host's configuration as it currently applies, mirroring the TOML.
- `def 'grimm get_config' [cfg_key: string]: nothing -> oneof<int, float, string, nothing>` One configuration setting by its dotted key; errors on an unknown key.
- `def 'grimm pin_config' [cfg_key: string, process_id: int, value: number]: nothing -> nothing` Hold a supervisor setting at a value for the lifetime of a process.
- `def 'print' [...rest: any, --no-newline (-n), --stderr (-e), --raw (-r)]: oneof<nothing, any> -> nothing` Print the given values to this call's stderr log.


## Example Usage
### Help
Tool parameter `command`:
```nu
interface help 'grammar:eye:git'
```
Tool result:
```nuon
```

# tools todo

### Eval
- `def 'rerun' [nonce: string, args: record, timeout_ms?: int]: nothing -> record<result: record, nonce: string>` Re-evaluate a cached `run()` body with fresh args.
- `def 'call' [namepath: string, args: record, timeout_ms?: int]: nothing -> record<result: record, nonce: string>` Invoke a committed rig function with typed args.
