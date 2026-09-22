use std::sync::Arc;

use hashbrown::HashMap;
use miette::NamedSource;
use shared::{Located, Result, Span};

use crate::{
    Expr, Ident, Stmt, StmtKind, components::Pat, errors::AlreadyInsideModule, visit::Visitor,
    walk_stmts,
};

/// A collection of statements.
#[derive(Debug, Clone)]
pub struct Ast {
    name: String,
    /// `NamedSource` for the file this ast came from. Cloned into any diagnostics emitted
    /// from post-parse checks (e.g. [`Self::module_name`]) so they render with snippets.
    src: NamedSource<Arc<str>>,
    stmts: Vec<Stmt>,
    /// Doc comments without their slashes, keyed by where the token after each starts.
    docs: HashMap<usize, String>,
}
impl Ast {
    /// Creates a new Ast with the given statements.
    pub(crate) fn new(
        name: String,
        src: NamedSource<Arc<str>>,
        stmts: Vec<Stmt>,
        docs: HashMap<usize, String>,
    ) -> Self {
        Self {
            name,
            src,
            stmts,
            docs,
        }
    }

    /// Consumes the Ast into its inner collection of statements.
    pub fn unpack(self) -> Vec<Stmt> {
        self.stmts
    }

    /// Get a reference to the ast's statements.
    pub fn stmts(&self) -> &[Stmt] {
        self.stmts.as_ref()
    }

    /// Get a mutable reference to the ast's statements.
    pub fn stmts_mut(&mut self) -> &mut Vec<Stmt> {
        &mut self.stmts
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The innermost node whose span covers `offset` (a byte offset into the source), with that
    /// span. Idents count as nodes, so a name wins over the expr or pat it sits in.
    pub fn node_at(&self, offset: usize) -> Option<(NodeId, Span)> {
        struct Innermost {
            offset: usize,
            found: Option<(NodeId, Span)>,
        }
        impl Innermost {
            fn consider(&mut self, id: NodeId, span: Span) {
                let covers = span.start <= self.offset && self.offset < span.end;
                let tighter = self
                    .found
                    .is_none_or(|(_, found)| span.len() <= found.len());
                if covers && tighter {
                    self.found = Some((id, span));
                }
            }
        }
        impl Visitor for Innermost {
            fn expr(&mut self, expr: &Expr) {
                self.consider(expr.id(), expr.span());
            }

            fn pat(&mut self, pat: &Pat) {
                self.consider(pat.id(), pat.span());
            }

            fn ident(&mut self, ident: &Ident) {
                self.consider(ident.id, ident.span());
            }
        }

        let mut innermost = Innermost {
            offset,
            found: None,
        };
        walk_stmts(&self.stmts, &mut innermost);
        innermost.found
    }

    /// The doc comment above the token starting at `position` (a byte offset into the source),
    /// without its slashes. An item's doc is at the start of its span.
    pub fn docs_for(&self, position: usize) -> Option<&str> {
        self.docs.get(&position).map(String::as_str)
    }

    pub fn module_name(&self) -> Result<Option<String>> {
        let module_name = self.stmts.first().and_then(|stmt| match stmt.kind() {
            StmtKind::Module(module) => Some(if module.name.lexeme == "@" {
                self.name.clone()
            } else {
                module.name.lexeme.clone()
            }),
            _ => None,
        });

        for stmt in self.stmts.iter().skip(usize::from(module_name.is_some())) {
            if let StmtKind::Module(module) = stmt.kind() {
                Err(AlreadyInsideModule {
                    src: self.src.clone(),
                    at: module.name.location.into(),
                })?
            }
        }

        Ok(module_name)
    }
}

/// A container for both expressions and statements.
///
/// NodeId now includes patterns, which aren't nodes in the sense that these are so...
/// yknow, that's bad
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Contains a [Stmt].
    Stmt(Stmt),
    /// Contains an [Expr].
    Expr(Expr),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
pub struct NodeId(u64);
impl NodeId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}
