# Notable Exclusions

Anything not described in this reference can be assumed not to exist in mimas -- the language is intentionally small. This page calls out a few absences that programmers coming from other languages are most likely to reach for, and why they're left out.

## Pointers & references

There are no pointers or references. They're a sharp tool that invites whole categories of instability, and guarding against that safely would demand a lifetime system as involved as Rust's. Like Python, mimas's answer is not to have them. If part of your project genuinely needs that level of control, that's a sign it belongs on the Rust side. ([Memory](./memory.md) covers the managed model that replaces them.)

## Union types

Arbitrary unions (`A | B`) undercut type inference and blur the line between static and dynamic typing in ways that get unpleasant fast. Where you'd reach for a union to write code generic over several types, mimas points you at a [pact](./pacts.md) instead. The only built-in "this or that" types are [options](./options.md) and [results](./error-handling.md), which are deliberate, well-behaved special cases.

## Generics

mimas has no user-facing generics. Abstracting over types is the job of [pacts](./pacts.md), which cover the common cases without the inference cost and complexity that a full generic system brings to an embeddable language.

## A borrow checker

There is no borrow checker -- garbage collection removes the need for one. Trading manual memory rigor for a managed runtime is exactly what buys mimas its forgiving, script-like feel.

## A `let` / `mut` split

There's no `mut`. Without a borrow checker there's little to gain from distinguishing mutable and immutable bindings, so the model is as simple as it gets: `let` is reassignable, `const` is not, and that's all. See [Variables & Constants](./variables.md).
