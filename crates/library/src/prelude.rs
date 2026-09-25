use macros::native;
use vm::{
    Ctx, RtErr, Val,
    anon::{self},
    api::Api,
    conversion::NeverReturn,
};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add(print);
    api.add(panic);
    api.add(todo);
    api.add(dbg);
}

/// Writes `value` to standard output, followed by a newline. A string prints as its plain text,
/// and any other value prints the same way it would inside an f-string.
///
/// ```mimas
/// print("hello");           // hello
/// print([1, 2, 3]);         // [1, 2, 3]
/// print(f"{1 + 2} apples"); // 3 apples
/// ```
#[native]
fn print<'gc>(ctx: Ctx<'gc>, value: Val<'gc>) {
    println!("{}", ctx.to_string(value));
}

/// Stops the script with a runtime error. The error reads `panic: ` followed by `msg`, or
/// `explicit panic` when there's no message.
///
/// `panic` never returns, and its [never type](../reference/special-types.md) lets it stand in for
/// a value of any type:
///
/// ```mimas
/// let config = ~{ port = 8080 };
/// let port = config["port"] ?? panic("no port configured");
/// ```
///
/// For a failure the caller should be able to recover from, return a
/// [result](../reference/error-handling.md#results) and `raise` instead.
#[native]
fn panic<'gc>(ctx: Ctx<'gc>, msg: Option<anon::T<'gc>>) -> Result<NeverReturn, RtErr> {
    Err(RtErr::Custom(
        msg.map(|msg| format!("panic: {}", ctx.to_string(msg.0)))
            .unwrap_or_else(|| "explicit panic".into()),
    ))
}

/// Stops the script with a runtime error that marks unfinished code. The error reads `todo: `
/// followed by `msg`, or just `todo` when there's no message.
///
/// Like [`panic`](#panic), it never returns, and it can stand in for a body you haven't written
/// yet:
///
/// ```mimas
/// fn load_save(path: str) -> ~{int} {
///     todo("read the save format")
/// }
/// ```
#[native]
fn todo<'gc>(ctx: Ctx<'gc>, msg: Option<anon::T<'gc>>) -> Result<NeverReturn, RtErr> {
    Err(match msg {
        Some(m) => RtErr::Custom(format!("todo: {}", ctx.to_string(m.0))),
        None => RtErr::Custom("todo".into()),
    })
}

/// Prints `value` to standard output in its debug form, then returns it. The line starts with
/// `dbg value:`, and strings keep their quotes.
///
/// Since it returns its argument, `dbg` can wrap any expression without changing the result:
///
/// ```mimas
/// let total = dbg(2 + 3) * 10; // prints `dbg value: 5`, and total is 50
/// let name = dbg("ada");       // prints `dbg value: "ada"`
/// ```
#[native]
fn dbg<'gc>(ctx: Ctx<'gc>, value: anon::T<'gc>) -> anon::T<'gc> {
    println!("dbg value: {}", ctx.display(value.0));
    value
}
