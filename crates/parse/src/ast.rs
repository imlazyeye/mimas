use std::sync::Arc;

use miette::NamedSource;
use shared::Result;

use crate::{Expr, Stmt, StmtKind, errors::AlreadyInsideModule};

/// A collection of statements.
#[derive(Debug, Clone)]
pub struct Ast {
    name: String,
    /// `NamedSource` for the file this ast came from. Cloned into any diagnostics emitted
    /// from post-parse checks (e.g. [`Self::module_name`]) so they render with snippets.
    src: NamedSource<Arc<str>>,
    stmts: Vec<Stmt>,
}
impl Ast {
    /// Creates a new Ast with the given statements.
    pub(crate) fn new(name: String, src: NamedSource<Arc<str>>, stmts: Vec<Stmt>) -> Self {
        Self { name, src, stmts }
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
