use core::{
    fmt::{self, Write as _},
    hint,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

use self::{line_buffered::LineBufferedConsole, sbi_debug::SbiDebugConsole};
use crate::spinlock::SpinMutex;

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

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut console = CONSOLE.try_lock();
    let mut dummy_console = LineBufferedConsole::new(SbiDebugConsole {});
    let console = console.as_deref_mut().unwrap_or(&mut dummy_console);

    let _ = writeln!(console, "\n\n!!! KERNEL PANIC !!!\n\n{info}\n\n");

    PANICKED.store(true, Ordering::Release);

    loop {}
}
