use crate::ir::{InstId, Ir, Local, Lower};
use parse::{Access, AccessKind, Expr, ExprKind};

pub(crate) trait Place {
    fn place(&self, ir: &mut Ir) -> Option<PlaceTarget>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaceTarget {
    Variable(Local),
    Index { array: InstId, index: InstId },
    Field { receiver: InstId, slot: u32 },
}

impl PlaceTarget {
    pub(crate) fn load(self, ir: &mut Ir) -> InstId {
        match self {
            PlaceTarget::Variable(variable) => ir.current().get_local(variable),
            PlaceTarget::Index { array, index } => {
                ir.current().get_index(array, index, AccessKind::Direct)
            }
            PlaceTarget::Field { receiver, slot } => {
                ir.current().get_field(receiver, slot, AccessKind::Direct)
            }
        }
    }

    pub(crate) fn store(self, ir: &mut Ir, value: InstId) -> InstId {
        match self {
            PlaceTarget::Variable(variable) => ir.current().set_local(variable, value),
            PlaceTarget::Index { array, index } => ir.current().set_index(array, index, value),
            PlaceTarget::Field { receiver, slot } => ir.current().set_field(receiver, slot, value),
        }
    }
}

impl Place for Access {
    fn place(&self, ir: &mut Ir) -> Option<PlaceTarget> {
        match self {
            Access::Identity { right: _ } => todo!(),
            Access::Dot {
                left,
                right,
                kind: _,
            } => {
                let recv = left.id();
                let receiver = left.lower(ir)?;
                let slot = match right.kind() {
                    ExprKind::Literal(parse::Literal::Int(i)) => u32::try_from(*i).unwrap(),
                    ExprKind::Ident(ident) => {
                        u32::try_from(ir.field_index(recv, &ident.lexeme)).unwrap()
                    }
                    _ => unreachable!(),
                };

                Some(PlaceTarget::Field { receiver, slot })
            }
            Access::DoubleColon { left: _, right: _ } => todo!(),
            Access::Square { left, key, kind: _ } => {
                let array = left.lower(ir)?;
                let index = key.lower(ir)?;

                Some(PlaceTarget::Index { array, index })
            }
        }
    }
}

impl Place for Expr {
    fn place(&self, ir: &mut Ir) -> Option<PlaceTarget> {
        // idents resolve off the enclosing Expr's NodeId -- Ident itself has no id
        if let ExprKind::Ident(_) = self.kind() {
            let dec = ir.node_dec(self.id());
            return Some(PlaceTarget::Variable(ir.local_for(dec)));
        }
        self.kind().place(ir)
    }
}

impl Place for ExprKind {
    fn place(&self, ir: &mut Ir) -> Option<PlaceTarget> {
        match self {
            ExprKind::Access(access) => access.place(ir),
            ExprKind::Ident(_) => unreachable!("idents are handled in `Place for Expr`"),
            _ => unreachable!("attempted to emit non-place expression as place"),
        }
    }
}
