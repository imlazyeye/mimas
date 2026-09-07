use shared::IdVec;

shared::id!(pub FlowId);

pub(crate) struct ControlFlow {
    flows: IdVec<FlowId, Flow>,
    flow_stack: Vec<FlowId>,
}

impl ControlFlow {
    pub(crate) fn new() -> Self {
        let mut cf = Self {
            flows: IdVec::new(),
            flow_stack: vec![],
        };
        cf.enter();
        cf
    }

    pub(crate) fn enter(&mut self) -> FlowId {
        let flow = Flow::Block(vec![]);
        let id = self.flows.push(flow);
        self.flow_stack.push(id);
        id
    }

    pub(crate) fn push(&mut self, flow: Flow) -> FlowId {
        let id = self.flows.push(flow);
        let Flow::Block(inner) = &mut self.flows[self.flow_stack.last().unwrap()] else {
            unreachable!()
        };
        inner.push(id);
        id
    }

    pub(crate) fn exit(&mut self) -> FlowId {
        self.flow_stack.pop().unwrap()
    }

    pub(crate) fn quantify(&self, id: &FlowId) -> Quantification {
        match &self.flows[id] {
            Flow::Block(cfs) => cfs
                .iter()
                .map(|x| self.quantify(x))
                .fold(Quantification::None, |a, x| a.higher(x)),
            Flow::Return => Quantification::Universal,
            Flow::Potential { positive, negative } => {
                self.quantify(positive).lower(self.quantify(negative))
            }
        }
    }
}

impl Default for ControlFlow {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub(crate) enum Flow {
    Block(Vec<FlowId>),
    Potential { positive: FlowId, negative: FlowId },
    Return,
}

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum Quantification {
    Universal,
    Existential,
    None,
}

impl Quantification {
    fn lower(self, o: Self) -> Self {
        match (self, o) {
            (Self::Universal, Self::Universal) => Self::Universal,
            (Self::None, Self::None) => Self::None,
            _ => Self::Existential,
        }
    }

    fn higher(self, o: Self) -> Self {
        match (self, o) {
            (Self::Universal, _) | (_, Self::Universal) => Self::Universal,
            (Self::Existential, _) | (_, Self::Existential) => Self::Existential,
            _ => Self::None,
        }
    }
}
