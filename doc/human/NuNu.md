> REIN HUMAN

Tools go under their subheaders with tool parameters directly beneath the header.

The rest of the content for each section is intended to be str included,
human rein file, as the literal tool schema documentation.

# MCP

MCP tool calls primarily use parameters and results in the text form of:
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
### Purview
### Channel
- `def 'interface channel open' []: nothing -> record<id: string>` Opens the Channel
- `def 'interface channel ready' []: nothing -> nothing` Verifies Channel activation and begins normal use.
- `def 'interface channel close' []: nothing -> nothing` Closes the channel

## Example Usage
### Help
Tool parameter `command`:
```nu
interface help 'grammar:eye:git'
```
Tool result:
```nuon
```
