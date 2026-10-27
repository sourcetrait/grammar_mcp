use crate::*;


pub const LINT_VIOLATION_CAP: usize = 3;


const ALLOWLIST_PATH_PREFIXES: &[&str] = &["/dev/", "/etc/", "/proc/", "/sys/"];

const DENYLIST_EXTERNALS: &[&str] = &[
    "awk", "bash", "cp", "date", "echo", "find", "grep", "jq", "ls", "nu", "rg", "rm", "sed", "sh",
    "sort", "which",
];

const REGEX_RECEIVERS: &[&str] = &[
    "find",
    "idx search",
    "parse",
    "split column",
    "split list",
    "split row",
    "str replace",
];

const PARSE_PATH_RECEIVERS: &[&str] = &["use", "overlay use", "source", "source-env"];


const MSG_HARDCODED_VARIABLE: &str = "hardcoded path literal; pass it in data";
const MSG_DENIED_COMMAND: &str = "denied external command; use a nushell builtin";


/// Lint a parsed block whose text begins at `text_start` in its working set.
pub fn lint_block(
    block: &nu::Block,
    ws: &nu::StateWorkingSet,
    text: &str,
    text_start: usize,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let _ = walk_block(block, ws, text, text_start, &mut diagnostics);
    diagnostics
}

fn push_diagnostic(diagnostics: &mut Vec<Diagnostic>, candidate: Diagnostic) -> ControlFlow<()> {
    if diagnostics.len() >= LINT_VIOLATION_CAP {
        ControlFlow::Break(())
    } else {
        diagnostics.push(candidate);
        ControlFlow::Continue(())
    }
}

fn walk_block(
    block: &nu::Block,
    ws: &nu::StateWorkingSet,
    body: &str,
    body_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> ControlFlow<()> {
    for p in &block.pipelines {
        for elem in &p.elements {
            walk_expr(&elem.expr, ws, body, body_start, diagnostics)?;
        }
    }
    ControlFlow::Continue(())
}

fn walk_expr(
    e: &nu::Expression,
    ws: &nu::StateWorkingSet,
    body: &str,
    body_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> ControlFlow<()> {
    match &e.expr {
        nu::Expr::Directory(s, _) | nu::Expr::Filepath(s, _) | nu::Expr::GlobPattern(s, _) => {
            check_path(s, e.span.start, body, body_start, diagnostics)?;
        }
        nu::Expr::String(s) | nu::Expr::RawString(s) => {
            check_path(s, e.span.start, body, body_start, diagnostics)?;
        }
        nu::Expr::StringInterpolation(parts) => {
            for part in parts {
                walk_expr(part, ws, body, body_start, diagnostics)?;
            }
        }
        nu::Expr::Call(call) => {
            let decl = ws.get_decl(call.decl_id);
            let name = decl.name();
            let regex_skip = REGEX_RECEIVERS.contains(&name)
                && call
                    .arguments
                    .iter()
                    .any(|a| matches!(a, nu::Argument::Named((n, _, _)) if n.item == "regex"));
            let parse_path_skip = PARSE_PATH_RECEIVERS.contains(&name);
            let mut positional_idx = 0usize;
            for arg in &call.arguments {
                match arg {
                    nu::Argument::Positional(ae) => {
                        let skip_this = positional_idx == 0
                            && (parse_path_skip
                                || (regex_skip
                                    && matches!(
                                        ae.expr,
                                        nu::Expr::String(_) | nu::Expr::RawString(_),
                                    )));
                        positional_idx += 1;
                        if skip_this {
                            continue;
                        }
                        walk_expr(ae, ws, body, body_start, diagnostics)?;
                    }
                    nu::Argument::Named((_n, _s, value)) => {
                        if let Some(ae) = value {
                            walk_expr(ae, ws, body, body_start, diagnostics)?;
                        }
                    }
                    nu::Argument::Unknown(ae) | nu::Argument::Spread(ae) => {
                        walk_expr(ae, ws, body, body_start, diagnostics)?;
                    }
                }
            }
        }
        nu::Expr::ExternalCall(head, ext_args) => {
            check_external_head(head, body, body_start, diagnostics)?;
            walk_expr(head, ws, body, body_start, diagnostics)?;
            for arg in ext_args.iter() {
                let inner = match arg {
                    nu::ExternalArgument::Regular(e) | nu::ExternalArgument::Spread(e) => e,
                };
                walk_expr(inner, ws, body, body_start, diagnostics)?;
            }
        }
        nu::Expr::FullCellPath(fcp) => {
            walk_expr(&fcp.head, ws, body, body_start, diagnostics)?;
        }
        nu::Expr::BinaryOp(lhs, op, rhs) => {
            walk_expr(lhs, ws, body, body_start, diagnostics)?;
            let is_regex_op = if let nu::Expr::Operator(operator) = &op.expr {
                matches!(
                    operator,
                    nu::Operator::Comparison(nu::Comparison::RegexMatch)
                        | nu::Operator::Comparison(nu::Comparison::NotRegexMatch),
                )
            } else {
                false
            };
            if !is_regex_op {
                walk_expr(rhs, ws, body, body_start, diagnostics)?;
            }
        }
        nu::Expr::Block(id)
        | nu::Expr::Closure(id)
        | nu::Expr::Subexpression(id)
        | nu::Expr::RowCondition(id) => {
            let b = ws.get_block(*id);
            walk_block(b, ws, body, body_start, diagnostics)?;
        }
        nu::Expr::UnaryNot(inner) => {
            walk_expr(inner, ws, body, body_start, diagnostics)?;
        }
        nu::Expr::Collect(_, inner) => {
            walk_expr(inner, ws, body, body_start, diagnostics)?;
        }
        nu::Expr::List(items) => {
            for item in items {
                match item {
                    nu::ListItem::Item(ae) | nu::ListItem::Spread(_, ae) => {
                        walk_expr(ae, ws, body, body_start, diagnostics)?;
                    }
                }
            }
        }
        nu::Expr::Table(t) => {
            for col in t.columns.iter() {
                walk_expr(col, ws, body, body_start, diagnostics)?;
            }
            for row in t.rows.iter() {
                for cell in row.iter() {
                    walk_expr(cell, ws, body, body_start, diagnostics)?;
                }
            }
        }
        nu::Expr::Record(items) => {
            for item in items {
                match item {
                    nu::RecordItem::Pair(k, v) => {
                        walk_expr(k, ws, body, body_start, diagnostics)?;
                        walk_expr(v, ws, body, body_start, diagnostics)?;
                    }
                    nu::RecordItem::Spread(_, e) => {
                        walk_expr(e, ws, body, body_start, diagnostics)?;
                    }
                }
            }
        }
        nu::Expr::Range(r) => {
            if let Some(e) = &r.from {
                walk_expr(e, ws, body, body_start, diagnostics)?;
            }
            if let Some(e) = &r.next {
                walk_expr(e, ws, body, body_start, diagnostics)?;
            }
            if let Some(e) = &r.to {
                walk_expr(e, ws, body, body_start, diagnostics)?;
            }
        }
        nu::Expr::MatchBlock(arms) => {
            for (pat, arm_body) in arms {
                walk_pattern(&pat.pattern, ws, body, body_start, diagnostics)?;
                walk_expr(arm_body, ws, body, body_start, diagnostics)?;
            }
        }
        nu::Expr::AttributeBlock(ab) => {
            for attr in &ab.attributes {
                walk_expr(&attr.expr, ws, body, body_start, diagnostics)?;
            }
            walk_expr(&ab.item, ws, body, body_start, diagnostics)?;
        }
        nu::Expr::GlobInterpolation(parts, _) => {
            for part in parts {
                walk_expr(part, ws, body, body_start, diagnostics)?;
            }
        }
        nu::Expr::Keyword(kw) => {
            walk_expr(&kw.expr, ws, body, body_start, diagnostics)?;
        }
        _ => {}
    }
    ControlFlow::Continue(())
}

fn walk_pattern(
    pat: &nu::Pattern,
    ws: &nu::StateWorkingSet,
    body: &str,
    body_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> ControlFlow<()> {
    match pat {
        nu::Pattern::Expression(e) => {
            walk_expr(e, ws, body, body_start, diagnostics)?;
        }
        nu::Pattern::Record(items) => {
            for (_key, sub) in items {
                walk_pattern(&sub.pattern, ws, body, body_start, diagnostics)?;
            }
        }
        nu::Pattern::List(subs) | nu::Pattern::Or(subs) => {
            for sub in subs {
                walk_pattern(&sub.pattern, ws, body, body_start, diagnostics)?;
            }
        }
        _ => {}
    }
    ControlFlow::Continue(())
}


fn check_path(
    s: &str,
    span_start: usize,
    body: &str,
    body_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> ControlFlow<()> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return ControlFlow::Continue(());
    }
    if trimmed.contains("://") {
        return ControlFlow::Continue(());
    }
    for prefix in ALLOWLIST_PATH_PREFIXES {
        if trimmed.starts_with(prefix) {
            return ControlFlow::Continue(());
        }
    }
    let flag = trimmed.starts_with('/')
        || trimmed.starts_with("~/")
        || trimmed.starts_with("./")
        || trimmed.starts_with("../");
    if flag {
        let body_offset = span_start.saturating_sub(body_start);
        let (line, col) = span_to_line_col(body, body_offset);
        let candidate = Diagnostic::error(
            "lint::hardcoded_variable",
            Some(Source {
                path: None,
                position: [line, col],
            }),
            MSG_HARDCODED_VARIABLE,
        );
        return push_diagnostic(diagnostics, candidate);
    }
    ControlFlow::Continue(())
}

fn check_external_head(
    head: &nu::Expression,
    body: &str,
    body_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> ControlFlow<()> {
    let name = match &head.expr {
        nu::Expr::GlobPattern(s, _)
        | nu::Expr::String(s)
        | nu::Expr::RawString(s)
        | nu::Expr::Filepath(s, _)
        | nu::Expr::Directory(s, _) => s.as_str(),
        _ => return ControlFlow::Continue(()),
    };
    let basename = name.rsplit('/').next().unwrap_or(name);
    if DENYLIST_EXTERNALS.contains(&basename) {
        let body_offset = head.span.start.saturating_sub(body_start);
        let (line, col) = span_to_line_col(body, body_offset);
        let candidate = Diagnostic::error(
            "lint::denied_command",
            Some(Source {
                path: None,
                position: [line, col],
            }),
            MSG_DENIED_COMMAND,
        );
        return push_diagnostic(diagnostics, candidate);
    }
    ControlFlow::Continue(())
}
