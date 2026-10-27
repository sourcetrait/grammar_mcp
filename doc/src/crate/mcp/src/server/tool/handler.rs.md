# handler.rs

## fn NuSh::tool_router
DENY IS ENFORCED AT ROUTER ASSEMBLY, not at call time, and that is the whole design: a denied
tool is ABSENT from `tools/list`, so its schema never enters the agent's context and there is
no denied-error loop to fall into. A call anyway fails at the rmcp layer as an unknown tool.

rmcp routers compose with `+` at runtime, which is what makes conditional assembly possible
at all rather than requiring a compile-time feature per tool.

Each of the four tools is deniable on its own, and there is NO implication between tokens -
denying `execute` does not deny `rerun` - so an operator's config lists exactly what it means. A
denied interface COMMAND leaves the tool standing: the command leaves the interface's parse
and its description instead (`server/interface/grammar.rs`), and denying `interface` removes
the tool whole.

The interface's description is written here, after its router is built, because it is
rendered from the deny set at runtime and the tool attribute takes only a constant.

## const MCP_DOC
User's `include/human/mcp/mcp.md`, verbatim: the envelope every tool answers, which he
keeps as the contract's own text, so the instructions an agent reads are his schema
rather than a restatement of it.

## fn get_info
`ServerCapabilities::builder().enable_tools()` is REQUIRED. `default()` advertises no tool
capability and Claude Code then skips `tools/list` entirely, which presents as a server with
no tools rather than as an error.

`env!("CARGO_PKG_VERSION")` must expand in THIS crate, so the version cannot be threaded in
from elsewhere.

The TITLE suffixes a non-default namespace so co-running namespace variants stay
tellable-apart in the client's own listing.

`instructions` opens with `MCP_DOC`, then renders the cache-path surface LIVE from the
resolved config rather than as a static string, so what it names is what this host actually
uses. The agent-facing operational craft beyond the envelope and those paths lives in the
skills, not here.

The same config answers `server/discover` in the 2026-07-28 era, rmcp building the
DiscoverResult from it, so a client that never sends `initialize` still gets the title and
the instructions.
