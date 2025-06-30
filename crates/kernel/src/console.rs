use core::{
    fmt::{self, Write as _},
    hint,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

use sbi::debug_console;

use crate::spinlock::SpinMutex;

static SBI_CONSOLE: SpinMutex<SbiConsole> = SpinMutex::new(SbiConsole {});
static PANICKED: AtomicBool = AtomicBool::new(false);

struct SbiConsole {}

impl fmt::Write for SbiConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut bytes = s.as_bytes();
        while !bytes.is_empty() {
            match debug_console::write(bytes) {
                // If no bytes were written, we should stop.
                Ok(0) => break,
                Ok(written) => bytes = &bytes[written..],
                Err(_) => return Err(fmt::Error),
            }
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    if PANICKED.load(Ordering::Acquire) {
        loop {
            // Spin forever to avoid further issues.
            hint::spin_loop();
        }
    }
    SBI_CONSOLE.lock().write_fmt(args).unwrap();
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
    let mut console = SBI_CONSOLE.try_lock();
    let mut dummy_console = SbiConsole {};
    let console = console.as_deref_mut().unwrap_or(&mut dummy_console);

    let _ = writeln!(console, "\n\n!!! KERNEL PANIC !!!\n\n{info}\n\n");

    PANICKED.store(true, Ordering::Release);

    loop {}
}
