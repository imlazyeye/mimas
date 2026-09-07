# Why mimas?

mimas is a young side project. It is not ready to replace your daily drivers or be used in a professional capacity, but it is ready for experimentation and collaborative development. To that end, I'd like to share why I made it and what it seeks to achieve.

## The gap to fill

Plenty of projects end up wanting two languages. You build the serious part -- the engine, the core, the thing that has to be fast and correct -- in something strict and heavy. Then, when you want to *move*, to describe behavior, try an idea, change something and see it a second later, you reach for a scripting layer on top.

Typically, this is where you have to make a trade. Scripting layers tend to commit hard to flexibility at the expense of safety. They're nimble and easy, but your trivial mistakes materialize as crashes for end-users, not errors for your compiles.

This has been a constant headache throughout my career in game development, and one with real consequences; games have enormous performance demands while also needing to be remarkably stable. There isn't much room to recover from an error mid-gameplay. You can wrap the unstable parts and pray, but catching an error you didn't model just kicks the can to a later, much more difficult to diagnose error. In the end, it feels as though I'm spending more time untangling cryptic crashes than I am actually making games.

mimas is my detox from those frustrations: a fun, snappy programming experience without gnashing my teeth at a crash from a typo the compiler _absolutely_ should have been capable of warning me about.

## Getting there

The path towards such a language necessitates a series of compromises. We're not going to produce some magical language that lets us program with our eyes closed while still fulfilling a [borrow checker](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html) -- instead, each part of the language needs to weigh the ease against safety. Will this frustrate me when I'm just trying to write something quick? Will this come back to haunt me at runtime? Each choice swings a little bit in one direction or the other; our ethos is to try to stay upright on that tightrope.

Ultimately, that has turned out to look a lot more like [Rust](https://www.rust-lang.org/) than [Python](https://www.python.org/), just with certain pieces removed. Some are to increase flexibility ([garbage collection](../reference/memory.md) instead of a borrow checker, no immutability beyond `const` declarations). Others are to reduce complexity, both for the compiler and the programmer (no references or pointers, no lifetimes to annotate). We start from safety and walk only as far away as we must to reach the flexibility we require.

## A small, clear target

None of this is a new wish. Wanting expressiveness, safety, and a fast loop all at once is a familiar itch, and people wiser than me have poured real work into pieces of it, even from inside Rust. The point isn't whether this project reaches that ideal, but that the chase is a worthwhile and rewarding place to spend time.

Thank you for stopping by to check mimas out! I'd gratefully welcome your thoughts and feedback over on our [GitHub](https://github.com/imlazyeye/mimas/issues/new).

-- [Gabe](https://github.com/imlazyeye)

```admonish faq title="No, but like, why 'mimas'?"
mimas is named after one of [Saturn's smallest moons](<https://en.wikipedia.org/wiki/Mimas_(moon)>) -- which is, in turn, named after a [giant](<https://en.wikipedia.org/wiki/Mimas_(Giant)>) from Greek mythology, a metaphor I think suits the project.

_Alternatively_, the reason is that it was the coolest available name I could find on crates.io. Whichever you prefer.
```
