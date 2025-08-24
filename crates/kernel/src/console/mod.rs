use core::{
    fmt::{self, Write as _},
    hint,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

use ansi_term::{Color, WithFg};

use self::{line_buffered::LineBufferedConsole, sbi_debug::SbiDebugConsole};
use crate::{
    cpu::{self, Cpu},
    sync::spinlock::SpinMutex,
    task::scheduler,
};

mod line_buffered;
mod sbi_debug;

static CONSOLE: SpinMutex<LineBufferedConsole<SbiDebugConsole>> =
    SpinMutex::new(LineBufferedConsole::new(SbiDebugConsole {}));
static PANICKED: AtomicBool = AtomicBool::new(false);

trait Console {
    type Error;
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<usize, Self::Error>;
}

pub fn print(args: fmt::Arguments) {
    if PANICKED.load(Ordering::Acquire) {
        loop {
            // Spin forever to avoid further issues.
            hint::spin_loop();
        }
    }
    CONSOLE.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*))
    };
}

struct OrUnknown<T>(Option<T>);

impl<T> fmt::Display for OrUnknown<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(value) = &self.0 {
            fmt::Display::fmt(value, f)
        } else {
            fmt::Display::fmt("<Unknown>", f)
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    PANICKED.store(true, Ordering::Release);
    let header = WithFg::new(Color::Red, "!!! KERNEL PANIC !!!");
    let cpuid = OrUnknown(cpu::try_current().map(Cpu::id));
    let taskid = OrUnknown(scheduler::try_current_task().map(|task| task.id()));
    let loc = OrUnknown(info.location());

    let mut console = CONSOLE.lock();
    let _ = writeln!(console);
    let _ = writeln!(console);
    let _ = writeln!(console, "{header}");
    let _ = writeln!(console);
    let _ = writeln!(console, "CPU:");
    let _ = writeln!(console, "  {cpuid}");
    let _ = writeln!(console);
    let _ = writeln!(console, "Task:");
    let _ = writeln!(console, "  {taskid}",);
    let _ = writeln!(console);
    let _ = writeln!(console, "Location:");
    let _ = writeln!(console, "  {loc}");
    let _ = writeln!(console);
    let _ = writeln!(console, "Message:");
    let _ = writeln!(console, "  {}", info.message());
    let _ = writeln!(console);
    loop {
        hint::spin_loop();
    }
}
