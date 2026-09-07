use api::AdtBinding;
use compile::BodyId;
use gc_arena::{Collect, Gc, GcWeak, Mutation, lock::RefLock};
use rustc_hash::FxHashMap;
use std::{any::TypeId, fmt::Write as _, ops, sync::Arc};

use crate::{
    Fields,
    val::{
        Array, Closure, ClosureData, Dict, DictMap, Instance, InstanceData, SharedStr, Str, Val,
    },
};

#[derive(Default, Debug)]
pub struct MimasBindings(pub FxHashMap<TypeId, AdtBinding>);

// SAFETY: TypeId + AdtBinding are 'static, no Gc handles inside.
unsafe impl<'gc> Collect<'gc> for MimasBindings {
    const NEEDS_TRACE: bool = false;
}

#[derive(Collect, Clone, Debug)]
#[collect(require_static)]
pub struct Frame {
    pub chunk: BodyId,
    pub ip: usize,
    pub return_reg: u32,
    pub base: usize,
}

#[derive(Collect)]
#[collect(no_drop)]
pub struct ThreadState<'gc> {
    pub regs: Vec<Val<'gc>>,
    pub frames: Vec<Frame>,
}

pub type Thread<'gc> = Gc<'gc, RefLock<ThreadState<'gc>>>;

#[derive(Copy, Clone, Collect)]
#[collect(no_drop)]
pub struct InternedStrings<'gc>(pub Gc<'gc, RefLock<FxHashMap<SharedStr, GcWeak<'gc, SharedStr>>>>);

impl<'gc> InternedStrings<'gc> {
    pub fn new(mc: &Mutation<'gc>) -> Self {
        InternedStrings(Gc::new(mc, RefLock::new(FxHashMap::default())))
    }

    pub fn intern(self, mc: &Mutation<'gc>, s: &str) -> Str<'gc> {
        if let Some(weak) = self.0.borrow().get(s)
            && let Some(gc) = weak.upgrade(mc)
        {
            return Str(gc);
        }
        let shared: SharedStr = Arc::from(s);
        let gc = Gc::new(mc, shared.clone());
        self.0.borrow_mut(mc).insert(shared, Gc::downgrade(gc));
        Str(gc)
    }
}

#[derive(Copy, Clone, Collect)]
#[collect(no_drop)]
pub struct State<'gc> {
    pub strings: InternedStrings<'gc>,
    pub thread: Thread<'gc>,
    pub natives: Gc<'gc, RefLock<Vec<Option<crate::native::NativeRef<'gc>>>>>,
    pub mimas_bindings: Gc<'gc, RefLock<MimasBindings>>,
    pub fixtures: Gc<'gc, crate::fixtures::Fixtures>,
}

impl<'gc> State<'gc> {
    pub fn new(mc: &Mutation<'gc>) -> Self {
        let thread = Gc::new(
            mc,
            RefLock::new(ThreadState {
                regs: Vec::new(),
                frames: Vec::new(),
            }),
        );
        let natives = Gc::new(mc, RefLock::new(Vec::new()));
        let mimas_bindings = Gc::new(mc, RefLock::new(MimasBindings::default()));
        let fixtures = Gc::new(mc, crate::fixtures::Fixtures::default());
        State {
            strings: InternedStrings::new(mc),
            thread,
            natives,
            mimas_bindings,
            fixtures,
        }
    }

    pub fn ctx(&'gc self, mutation: &'gc Mutation<'gc>) -> Ctx<'gc> {
        Ctx {
            mutation,
            state: self,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Ctx<'gc> {
    mutation: &'gc Mutation<'gc>,
    state: &'gc State<'gc>,
}

impl<'gc> ops::Deref for Ctx<'gc> {
    type Target = Mutation<'gc>;
    fn deref(&self) -> &Self::Target {
        self.mutation
    }
}

impl<'gc> Ctx<'gc> {
    pub fn state(self) -> &'gc State<'gc> {
        self.state
    }

    pub fn mutation(self) -> &'gc Mutation<'gc> {
        self.mutation
    }

    pub fn intern(self, s: &str) -> Str<'gc> {
        self.state.strings.intern(self.mutation, s)
    }

    /// Fetch the per-Vm fixture of type `T`, creating it on first access -- see the
    /// [fixtures](crate::fixtures) module docs for the full story.
    pub fn fixture<T: crate::fixtures::Fixture>(self) -> &'gc T {
        self.state.fixtures.get::<T>()
    }

    pub fn thread(self) -> Thread<'gc> {
        self.state.thread
    }

    pub fn new_array(self, items: Vec<Val<'gc>>) -> Array<'gc> {
        Array(Gc::new(self.mutation, RefLock::new(items)))
    }

    pub fn new_dict(self, items: DictMap<'gc>) -> Dict<'gc> {
        Dict(Gc::new(self.mutation, RefLock::new(items)))
    }

    pub fn new_instance(self, struct_id: u32, fields: Fields<'gc>) -> Instance<'gc> {
        Instance(Gc::new(
            self.mutation,
            RefLock::new(InstanceData { struct_id, fields }),
        ))
    }

    pub fn new_closure(self, function: BodyId, captures: Vec<Val<'gc>>) -> Closure<'gc> {
        Closure(Gc::new(self.mutation, ClosureData { function, captures }))
    }

    // fresh allocation per container so the result shares no mutable state with `value`. scalars,
    // interned strs, fns and closures are copied by handle (immutable / callable). recurses like
    // `display`, so it shares display's no-cycles assumption.
    pub fn deep_clone(self, value: Val<'gc>) -> Val<'gc> {
        match value {
            Val::Array(a) => {
                let items: Vec<Val<'gc>> =
                    a.0.borrow().iter().map(|&v| self.deep_clone(v)).collect();
                Val::Array(self.new_array(items))
            }
            Val::Dict(d) => {
                let items: DictMap<'gc> =
                    d.0.borrow()
                        .iter()
                        .map(|(&k, &v)| (k, self.deep_clone(v)))
                        .collect();
                Val::Dict(self.new_dict(items))
            }
            Val::Instance(i) => {
                let (struct_id, fields) = {
                    let inst = i.0.borrow();
                    let fields: Vec<Val<'gc>> =
                        inst.fields.iter().map(|&v| self.deep_clone(v)).collect();
                    (inst.struct_id, fields)
                };
                Val::Instance(self.new_instance(struct_id, Fields::new(fields)))
            }
            other => other,
        }
    }

    /// Render a value being *shown* -- strings are quoted (`"x"`). Use this for anything
    /// inspection-shaped: dbg output, test harnesses, error reports.
    pub fn display(self, value: Val<'gc>) -> String {
        let mut out = String::new();
        self.render_into(&mut out, value, true);
        out
    }

    /// Render a value as *text* -- a string yields its own contents, unquoted. This is what
    /// f-strings and `print` use; every non-string renders the same as [Ctx::display].
    pub fn to_string(self, value: Val<'gc>) -> String {
        let mut out = String::new();
        self.render_into(&mut out, value, false);
        out
    }

    /// [Ctx::to_string], appending into an existing buffer (the f-string builder).
    pub fn to_string_into(self, out: &mut String, value: Val<'gc>) {
        self.render_into(out, value, false);
    }

    fn render_into(self, out: &mut String, value: Val<'gc>, quote: bool) {
        match value {
            Val::Null => out.push_str("null"),
            Val::Bool(b) => out.push_str(if b { "true" } else { "false" }),
            // — alloc
            Val::Int(i) => out.push_str(itoa::Buffer::new().format(i)),
            Val::Float(f) => {
                let _ = write!(out, "{f}");
            }
            Val::Str(s) => {
                if quote {
                    let _ = write!(out, "{:?}", s.as_str());
                } else {
                    out.push_str(s.as_str());
                }
            }
            // nested strings always quote -- `quote` only governs the top-level value, so
            // `print("x")` is bare while `print(["x"])` renders ["x"]
            Val::Array(a) => {
                out.push('[');
                for (i, &v) in a.0.borrow().iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.render_into(out, v, true);
                }
                out.push(']');
            }
            Val::Dict(d) => {
                // todo: this is still allocing
                let entries =
                    d.0.borrow()
                        .iter()
                        .map(|(k, v)| format!("{} = {}", k.as_str(), self.display(*v)))
                        .collect::<Vec<_>>()
                        .join(", ");
                let _ = write!(out, "~{{{entries}}}");
            }
            Val::Fn(b) => {
                let _ = write!(out, "<fn @{}>", b.index());
            }
            Val::Closure(c) => {
                let _ = write!(
                    out,
                    "<closure @{} captures={}>",
                    c.0.function.index(),
                    c.0.captures.len()
                );
            }
            Val::Raised(s) => {
                let _ = write!(out, "<raised {:?}>", s.as_str());
            }
            Val::Instance(i) => {
                let inst = i.0.borrow();
                let entries = inst
                    .fields
                    .iter()
                    .copied()
                    .map(|v| self.display(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = write!(out, "@{} {{ {} }}", inst.struct_id, entries);
            }
        }
    }
}
