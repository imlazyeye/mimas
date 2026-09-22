use crate::{
    Access, Expr, ExprKind, FStringPart, FieldKey, Ident, Item, ItemKind, Literal, PactItem, Stmt,
    StmtKind, Use,
    components::{Binding, Pat, PatKind},
};

pub trait Visitor {
    fn item(&mut self, _item: &Item) {}
    fn expr(&mut self, _expr: &Expr) {}
    fn pat(&mut self, _pat: &Pat) {}
    fn ident(&mut self, _ident: &Ident) {}
}

pub fn walk_stmts(stmts: &[Stmt], visitor: &mut impl Visitor) {
    for stmt in stmts {
        walk_stmt(stmt, visitor);
    }
}

pub fn walk_stmt(stmt: &Stmt, visitor: &mut impl Visitor) {
    match stmt.kind() {
        StmtKind::Let(l) => {
            walk_pat(&l.left, visitor);
            walk_expr(&l.right, visitor);
            if let Some(else_branch) = &l.else_branch {
                walk_expr(else_branch, visitor);
            }
        }
        StmtKind::Module(_) => {}
        StmtKind::Assignment(assignment) => {
            walk_expr(&assignment.left, visitor);
            walk_expr(&assignment.right, visitor);
        }
        StmtKind::Expr(expr) => walk_expr(expr, visitor),
        StmtKind::Item(item) => walk_item(item, visitor),
    }
}

pub fn walk_item(item: &Item, visitor: &mut impl Visitor) {
    visitor.item(item);
    match item.kind() {
        ItemKind::Function(function) => {
            visitor.ident(&function.name);
            for parameter in &function.parameters {
                walk_binding(parameter, visitor);
            }
            walk_expr(&function.body, visitor);
        }
        ItemKind::Impl(imp) => {
            visitor.ident(&imp.target);
            if let Some(pact) = &imp.pact {
                visitor.ident(pact);
            }
            for item in &imp.items {
                walk_item(item, visitor);
            }
        }
        ItemKind::Pact(pact) => {
            visitor.ident(&pact.name);
            for item in &pact.items {
                if let PactItem::Fn {
                    parameters,
                    default,
                    ..
                } = item
                {
                    for parameter in parameters {
                        walk_binding(parameter, visitor);
                    }
                    if let Some(default) = default {
                        walk_expr(default, visitor);
                    }
                }
            }
        }
        ItemKind::Const(con) => {
            visitor.ident(&con.left);
            walk_expr(&con.right, visitor);
        }
        ItemKind::Struct(struc) => visitor.ident(&struc.name),
        ItemKind::Enum(en) => visitor.ident(&en.head),
        ItemKind::Use(us) => {
            let (path, items): (&[Ident], &[Ident]) = match us {
                Use::Singular(path, item) => (path, std::slice::from_ref(item)),
                Use::Multi(path, items) => (path, items),
                Use::All(path) => (path, &[]),
            };
            for ident in path.iter().chain(items) {
                visitor.ident(ident);
            }
        }
        ItemKind::Poison(_) => {}
    }
}

pub fn walk_expr(expr: &Expr, visitor: &mut impl Visitor) {
    visitor.expr(expr);
    match expr.kind() {
        ExprKind::Absolve(absolve) => {
            walk_expr(&absolve.left, visitor);
            walk_expr(&absolve.handler, visitor);
        }
        ExprKind::Access(access) => match access {
            Access::Identity { right } => visitor.ident(right),
            Access::Dot { left, right, .. } => {
                walk_expr(left, visitor);
                walk_expr(right, visitor);
            }
            Access::DoubleColon { left, right } => {
                walk_expr(left, visitor);
                visitor.ident(right);
            }
            Access::Square { left, key, .. } => {
                walk_expr(left, visitor);
                walk_expr(key, visitor);
            }
        },
        ExprKind::Block(block) => {
            walk_stmts(&block.body, visitor);
            if let Some(yielded) = &block.yielded_expr {
                walk_expr(yielded, visitor);
            }
        }
        ExprKind::Break(brk) => {
            if let Some(value) = &brk.value {
                walk_expr(value, visitor);
            }
        }
        ExprKind::Call(call) => {
            walk_expr(&call.left, visitor);
            for argument in &call.arguments {
                if let Some(name) = &argument.name {
                    visitor.ident(name);
                }
                walk_expr(&argument.value, visitor);
            }
        }
        ExprKind::Closure(closure) => {
            for parameter in &closure.parameters {
                walk_binding(parameter, visitor);
            }
            walk_expr(&closure.body, visitor);
        }
        ExprKind::Collect(collect) => walk_expr(&collect.value, visitor),
        ExprKind::Equality(equality) => {
            walk_expr(&equality.left, visitor);
            walk_expr(&equality.right, visitor);
        }
        ExprKind::Evaluation(evaluation) => {
            walk_expr(&evaluation.left, visitor);
            walk_expr(&evaluation.right, visitor);
        }
        ExprKind::For(fo) => {
            walk_pat(&fo.binding, visitor);
            walk_expr(&fo.iterator, visitor);
            walk_expr(&fo.body, visitor);
        }
        ExprKind::FString(fstring) => {
            for part in &fstring.parts {
                if let FStringPart::Expr(expr) = part {
                    walk_expr(expr, visitor);
                }
            }
        }
        ExprKind::Grouping(grouping) => walk_expr(&grouping.inner, visitor),
        ExprKind::If(i) => {
            if let Some(binding) = &i.binding {
                walk_pat(binding, visitor);
            }
            walk_expr(&i.condition, visitor);
            walk_expr(&i.main_body, visitor);
            if let Some(else_expr) = &i.else_expr {
                walk_expr(else_expr, visitor);
            }
        }
        ExprKind::In(i) => {
            walk_expr(&i.left, visitor);
            walk_expr(&i.right, visitor);
        }
        ExprKind::Literal(literal) => match literal {
            Literal::Array(exprs) | Literal::Tuple(exprs) => {
                for expr in exprs {
                    walk_expr(expr, visitor);
                }
            }
            Literal::Dictionary(fields) => {
                for (key, value) in fields {
                    visitor.ident(key);
                    walk_expr(value, visitor);
                }
            }
            Literal::Struct(struc) => {
                walk_expr(&struc.name, visitor);
                for (key, value) in &struc.fields {
                    if let FieldKey::Ident(key) = key {
                        visitor.ident(key);
                    }
                    walk_expr(value, visitor);
                }
            }
            _ => {}
        },
        ExprKind::Logical(logical) => {
            walk_expr(&logical.left, visitor);
            walk_expr(&logical.right, visitor);
        }
        ExprKind::Loop(lop) => walk_expr(&lop.body, visitor),
        ExprKind::Match(mat) => {
            walk_expr(&mat.identity, visitor);
            for case in &mat.cases {
                walk_pat(case.pat(), visitor);
                if let Some(guard) = case.guard() {
                    walk_expr(guard, visitor);
                }
                walk_expr(case.body(), visitor);
            }
        }
        ExprKind::Coalescence(coalescence) => {
            walk_expr(&coalescence.left, visitor);
            walk_expr(&coalescence.right, visitor);
        }
        ExprKind::Raise(raise) => walk_expr(&raise.value, visitor),
        ExprKind::Range(range) => {
            walk_expr(&range.start, visitor);
            walk_expr(&range.end, visitor);
        }
        ExprKind::Return(ret) => {
            if let Some(value) = &ret.value {
                walk_expr(value, visitor);
            }
        }
        ExprKind::Unary(unary) => walk_expr(&unary.right, visitor),
        ExprKind::Unwrap(unwrap) => walk_expr(&unwrap.expr, visitor),
        ExprKind::While(whil) => {
            if let Some(binding) = &whil.binding {
                walk_pat(binding, visitor);
            }
            walk_expr(&whil.header, visitor);
            walk_expr(&whil.body, visitor);
        }
        ExprKind::Ident(ident) => visitor.ident(ident),
        ExprKind::Continue(_) | ExprKind::Poison(_) => {}
    }
}

pub fn walk_pat(pat: &Pat, visitor: &mut impl Visitor) {
    visitor.pat(pat);
    match pat.kind() {
        PatKind::Tuple(pats) | PatKind::Or(pats) => {
            for pat in pats {
                walk_pat(pat, visitor);
            }
        }
        PatKind::Struct(expr, fields) => {
            walk_expr(expr, visitor);
            for pat in fields.values() {
                walk_pat(pat, visitor);
            }
        }
        PatKind::TupleVariant(expr, pats) => {
            walk_expr(expr, visitor);
            for pat in pats {
                walk_pat(pat, visitor);
            }
        }
        PatKind::Variant(expr) => walk_expr(expr, visitor),
        PatKind::NullBind(pat) => walk_pat(pat, visitor),
        PatKind::Ident(ident) => visitor.ident(ident),
        PatKind::Literal(_) | PatKind::Poison(_) => {}
    }
}

pub fn walk_binding(binding: &Binding, visitor: &mut impl Visitor) {
    walk_pat(&binding.left, visitor);
    if let Some(right) = &binding.right {
        walk_expr(right, visitor);
    }
}
