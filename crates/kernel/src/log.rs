use alloc::sync::Arc;
use core::{fmt, panic::Location};

use ansi_term::{Color, WithFg};

use crate::{
    cpu::{self, Cpu},
    interrupt::{
        self,
        timer::{self, Instant},
    },
    task::{Task, scheduler},
};

macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::log::log($level, format_args!($($arg)*));
    };
}

#[expect(unused_macros)]
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
    let interrupt_guard = interrupt::push_disabled();
    let now = TimeFormat(timer::try_now());
    let level = LevelFormat(level);
    let task = TaskFormat(scheduler::try_current_task());
    let cpu = CpuFormat(cpu::try_current());
    let location = LocationFormat(Location::caller());
    interrupt_guard.pop();

    println!("{now} [{task}@{cpu}] {level} {message} {location}");
}

#[expect(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug)]
struct TimeFormat(Option<Instant>);

impl fmt::Display for TimeFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(now) => {
                let dur = now.duration_since_epoc();
                write!(f, "{}.{:09}", dur.as_secs(), dur.subsec_nanos())
            }
            None => write!(f, "{0}.{1:09}", 0, 0),
        }
    }
}

#[derive(Debug)]
struct TaskFormat(Option<Arc<Task>>);

impl fmt::Display for TaskFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(task) => write!(f, "{}", task.id()),
            None => write!(f, "S"),
        }
    }
}

#[derive(Debug)]
struct CpuFormat<'a>(Option<&'a Cpu>);

impl fmt::Display for CpuFormat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(cpu) => write!(f, "{}", cpu.id()),
            None => write!(f, "?"),
        }
    }
}

#[derive(Debug)]
struct LevelFormat(LogLevel);

impl fmt::Display for LevelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let color = match self.0 {
            LogLevel::Trace => Color::Purple,
            LogLevel::Debug => Color::Blue,
            LogLevel::Info => Color::Green,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Error => Color::Red,
        };
        let msg = match self.0 {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => " INFO",
            LogLevel::Warn => " WARN",
            LogLevel::Error => "ERROR",
        };
        write!(f, "{}", WithFg::new(color, msg))
    }
}

#[derive(Debug)]
struct LocationFormat<'a>(&'a Location<'a>);

impl fmt::Display for LocationFormat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let color = Color::DarkGray;
        let location = format_args!("({})", self.0);
        write!(f, "{}", WithFg::new(color, location))
    }
}
