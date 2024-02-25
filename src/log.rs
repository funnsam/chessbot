macro_rules! dbg {
    ($fmt: tt $($args: tt)*) => {
        eprintln!(concat!("\x1b[90mDebug: ", $fmt, "\x1b[0m") $($args)*);
    };
}

macro_rules! info {
    ($fmt: tt $($args: tt)*) => {
        println!(concat!("\x1b[1;32mInfo:\x1b[0m ", $fmt) $($args)*);
    };
}

macro_rules! warn {
    ($fmt: tt $($args: tt)*) => {
        println!(concat!("\x1b[1;33mWarn:\x1b[0m ", $fmt) $($args)*);
    };
}

macro_rules! error {
    ($fmt: tt $($args: tt)*) => {
        println!(concat!("\x1b[1;31mError:\x1b[0m ", $fmt) $($args)*);
    };
}
