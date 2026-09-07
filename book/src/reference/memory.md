# Memory

mimas is **garbage collected**. Memory is managed for you by a collector built on [`gc-arena`](https://github.com/kyren/gc-arena): there is no manual allocation, no `free`, no pointers, and no lifetimes to thread through your code. You create values and use them, and the runtime reclaims what you no longer reach.

This is a deliberate trade: a forgiving runtime paired with a strict compiler. The static analysis mimas does is spent on correctness -- types, exhaustiveness, null-safety -- not on tracking who owns what. The goal is a language that feels like scripting to write while still catching real mistakes before you run.

```admonish note title="Where the heavy lifting belongs"
If a piece of your project needs ownership, lifetimes, or manual control over memory, that's a sign it belongs on the Rust side of the boundary. mimas is the scripting layer; Rust is the place for the parts that demand that rigor. See [Extension with Rust](../extension-with-rust.md).
```

```admonish danger title="Running out of memory"
The collector manages reclamation, not quotas. A script can exhaust the process's memory, and when it does, the process aborts -- there is no error a script (or its host) can catch at that point. If you embed mimas and run scripts you don't control, account for this at the host level for now. We intend to offer a per-Vm memory budget that surfaces exhaustion as an ordinary fault, but it does not exist today.
```
