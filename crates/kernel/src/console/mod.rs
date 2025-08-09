use core::{
    fmt::{self, Write as _},
    hint,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

use ansi_term::{Color, WithFg};

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
    PANICKED.store(true, Ordering::Release);
    let mut console = CONSOLE.lock();
    let _ = writeln!(
        console,
        "\n\n{}\n\n{info}\n\n",
        WithFg::new(Color::Red, "!!! KERNEL PANIC !!!")
    );
    loop {
        hint::spin_loop();
    }
}
