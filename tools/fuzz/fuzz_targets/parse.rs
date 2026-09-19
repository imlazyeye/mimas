#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = mimas::compile_source(&String::from_utf8_lossy(data));
});
