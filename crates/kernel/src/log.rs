use core::fmt;

use crate::{
    cpu::{self, Cpu},
    interrupt::timer,
};

macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::log::log($level, format_args!($($arg)*));
    };
}

macro_rules! trace {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Trace, $($arg)*);
    };
}

#[expect(unused_macros)]
macro_rules! debug {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Debug, $($arg)*);
    };
}

macro_rules! info {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Info, $($arg)*);
    };
}

#[expect(unused_macros)]
macro_rules! warn {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Warn, $($arg)*);
    };
}

#[expect(unused_macros)]
macro_rules! error {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Error, $($arg)*);
    };
}

#[track_caller]
pub fn log(level: LogLevel, message: fmt::Arguments) {
    let now = timer::now();
    if let Some(cpuid) = cpu::try_current().map(Cpu::id) {
        println!("{now:?} [{cpuid}] {} {}", LevelFormat(level), message);
    } else {
        println!("{now:?} [?] {} {}", LevelFormat(level), message);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

struct LevelFormat(LogLevel);

impl fmt::Display for LevelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let color = match self.0 {
            LogLevel::Trace => 35,
            LogLevel::Debug => 34,
            LogLevel::Info => 32,
            LogLevel::Warn => 33,
            LogLevel::Error => 31,
        };
        let msg = match self.0 {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => " INFO",
            LogLevel::Warn => " WARN",
            LogLevel::Error => "ERROR",
        };
        write!(f, "\x1B[{color};1m{msg}\x1B[0m")
    }
}
