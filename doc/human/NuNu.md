# NuNu
> REIN HUMAN

Tools go under their subheaders with tool parameters directly beneath the header.

The rest of the content for each section is intended to be str included,
human rein file, as the literal tool schema documentation.

## mcp tool call: grammar/nu
  definition: string
  data: string

Executes the provided Nu language definition against the NUON `$data` provided.

Language contract (requirements):
- One top-level definition named `execute` that uses a Nu infix type signature
- Single parameter named `data` that must be typed as:
  - `nothing`
  - `record<TYPEDEF>` with a strict type definition
  - `table<TYPEDEF>` with a strict type definition
- The infix input type must always be `nothing`
- The infix return type must be typed as:
- `record<TYPEDEF> with a strict type definition
- `table<TYPEDEF> with a strict type definition
- Values must be passed as data.
  - Not hard-coded in the body of the definition.
  - This especially includes paths.

This contract is enforced and will error if broken.
Aggressive type-checking will error on language contract violations.
Aggressive linting will error if values are hard-coded in the body.

Execution can be either stateless (default) or stateful (`--env`).

Stateless operation runs across a pool of isolated stateless workers.

Stateful execution is queued against a single stateful worker and persists
across the worker's long-term Nu environment.

## Examples
### Stateless execution
Tool parameter `definition`:
```nu
def execute [data: table<num: int>]: nothing -> table<id: int, name: string> {
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
[[id, name]; [0, zero], [1, "10"]]
```
### Stateful execution
Tool parameter `definition`:
```nu
def --env execute [data: record<dir: directory, file: path>]: nothing -> record<id: int, name: string> {
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
{id: 3, name: foobar}
```

## mcp tool call: grammar/interface
 command: string

Interfaces directly with the Grammar MCP.

Runs a single MCP `interface` Nu Command in an isolated Nu context without
access to any other commands.

### Command Signatures
#### General
- `def 'interface help' [namepath: string]: nothing -> record<>` Displays equipment documentation
- `def 'interface info' []: nothing -> record<>` MCP details, Equipment signatures within current purview
#### Equip
#### Purview
#### Channel
- `def 'interface channel open' []: nothing -> record<id: string>` Opens the Channel
- `def 'interface channel ready' []: nothing -> nothing` Verifies Channel activation and begins normal use.
- `def 'interface channel close' []: nothing -> nothing` Closes the channel

### Example Usage
#### Help
Tool parameter `command`:
```nu
interface help 'grammar:eye:git'
```
Tool result:
```nuon
```
