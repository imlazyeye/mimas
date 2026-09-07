// catches stray rust panics from the pipeline so users see a styled diagnostic
// instead of a raw stack dump. install_hook stashes the backtrace into a
// thread-local; catch() pulls it back out after unwinding.

use backtrace::Backtrace;
use color_backtrace::BacktracePrinter;
use colored::Colorize;
use std::{
    cell::{Cell, RefCell},
    panic::{self, AssertUnwindSafe, catch_unwind},
};

pub struct IceInfo {
    pub message: String,
    pub location: Option<String>,
    pub backtrace: Backtrace,
}

pub struct IceReport {
    pub stage: &'static str,
    pub info: IceInfo,
}

thread_local! {
    static LAST: RefCell<Option<IceInfo>> = const { RefCell::new(None) };
    static DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub fn install_hook() {
    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&'static str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".into());
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        let backtrace = Backtrace::new();
        LAST.with(|cell| {
            *cell.borrow_mut() = Some(IceInfo {
                message,
                location,
                backtrace,
            });
        });
        // outside catch() we still want the user to see *something* -- fall back
        // to the previous hook so unexpected panics in the cli itself print normally.
        if DEPTH.with(|d| d.get()) == 0 {
            prev(info);
        }
    }));
}

pub fn catch<F, R>(stage: &'static str, f: F) -> Result<R, IceReport>
where
    F: FnOnce() -> R,
{
    DEPTH.with(|d| d.set(d.get() + 1));
    let result = catch_unwind(AssertUnwindSafe(f));
    DEPTH.with(|d| d.set(d.get() - 1));
    result.map_err(|_| IceReport {
        stage,
        info: LAST
            .with(|c| c.borrow_mut().take())
            .unwrap_or_else(|| IceInfo {
                message: "<panic captured without metadata>".into(),
                location: None,
                backtrace: Backtrace::new(),
            }),
    })
}

impl IceReport {
    pub fn emit(&self) {
        eprintln!();
        eprintln!(
            "{}: mimas panicked during {}",
            "internal compiler error".bright_red().bold(),
            self.stage.bold(),
        );
        eprintln!("  {} {}", "->".bright_red(), self.info.message);
        if let Some(loc) = &self.info.location {
            eprintln!("  {} {}", "at".bright_black(), loc.bright_black());
        }
        eprintln!();
        eprintln!(
            "{}",
            "this is a bug in mimas, not your code."
                .italic()
                .bright_black()
        );
        eprintln!(
            "{}",
            "please report it with the source you ran and the trace below."
                .italic()
                .bright_black()
        );
        eprintln!();
        // color-backtrace's default filter only trims edges (post-panic + runtime
        // init); it leaves /rustc and rustlib frames in the middle. retain only
        // frames whose source file lives in our workspace.
        let mut printer = BacktracePrinter::new();
        if std::env::var_os("MIMAS_RAW_BACKTRACE").is_none() {
            printer = printer.add_frame_filter(Box::new(|frames| {
                frames.retain(|f| {
                    let in_workspace = f.filename.as_deref().is_some_and(|p| {
                        let s = p.to_string_lossy();
                        s.contains("/crates/") || s.contains("/examples/")
                    });
                    in_workspace && !f.name.as_deref().unwrap_or("").contains("mimas::ice::")
                });
            }));
        }
        let out = color_backtrace::default_output_stream();
        let _ = printer.print_trace(&self.info.backtrace, &mut out.lock());
        if std::env::var_os("MIMAS_RAW_BACKTRACE").is_none() {
            eprintln!(
                "{}",
                "  (set MIMAS_RAW_BACKTRACE=1 for the full untrimmed trace)"
                    .italic()
                    .bright_black()
            );
        }
    }
}
