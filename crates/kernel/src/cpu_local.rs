use alloc::vec::Vec;
use core::{alloc::Layout, arch::asm, ptr};

use spin::Once;

use crate::cpu::{self, Cpuid};

macro_rules! cpu_local {
    () => {};
    ($(#[$attr:meta])* $vis:vis static $name:ident : $t:ty = $init:expr; $($rest:tt)*) => {
        $(#[$attr])* $vis const $name: $crate::cpu_local::CpuLocal<$t> = {
            #[unsafe(link_section = ".percpu")]
            static $name: $t = $init;

            $crate::cpu_local::CpuLocal::new(&raw const $name)
        };
        cpu_local!($($rest)*);
    };
}

unsafe extern "C" {
    #[link_name = "__onix_percpu_start"]
    static mut PERCPU_START: u8;
    #[link_name = "__onix_percpu_end"]
    static mut PERCPU_END: u8;
}

pub struct CpuLocal<T> {
    template: *const T,
}

unsafe impl<T> Sync for CpuLocal<T> {}

impl<T> CpuLocal<T> {
    pub const fn new(template: *const T) -> Self {
        Self { template }
    }

    pub fn get(&self) -> &T {
        self.try_get().unwrap()
    }

    pub fn try_get(&self) -> Option<&T> {
        let tp = get_thread_pointer();
        if tp.is_null() {
            return None;
        }
        let template_offset = unsafe {
            self.template
                .cast::<u8>()
                .byte_offset_from_unsigned(&raw const PERCPU_START)
        };
        let data = unsafe {
            ptr::with_exposed_provenance::<T>(tp.addr() + template_offset)
                .as_ref()
                .unwrap()
        };
        Some(data)
    }
}

fn get_thread_pointer() -> *mut u8 {
    let tp: *mut u8;
    unsafe {
        asm!("mv {0}, tp", out(reg) tp);
    }
    tp
}

fn set_thread_pointer(tp: *mut u8) {
    unsafe {
        asm!("mv tp, {0}", in(reg) tp);
    }
}

static ARENAS: Once<Vec<Arena>> = Once::new();

struct Arena {
    cpuid: Cpuid,
    arena: *mut u8,
}

unsafe impl Sync for Arena {}
unsafe impl Send for Arena {}

pub fn init() {
    let arena_size =
        unsafe { (&raw const PERCPU_END).offset_from_unsigned(&raw const PERCPU_START) };
    let alloc_size = usize::max(arena_size, 16);

    ARENAS.call_once(|| {
        cpu::get_all()
            .iter()
            .map(|cpu| {
                let cpuid = cpu.id();
                let arena = unsafe {
                    alloc::alloc::alloc(Layout::from_size_align(alloc_size, 16).unwrap())
                };
                assert!(!arena.is_null());
                unsafe {
                    arena.copy_from_nonoverlapping(&raw const PERCPU_START, arena_size);
                }
                Arena { cpuid, arena }
            })
            .collect()
    });
}

pub fn apply(cpuid: Cpuid) {
    assert!(get_thread_pointer().is_null(), "{:p}", get_thread_pointer());
    let arena = ARENAS
        .get()
        .unwrap()
        .iter()
        .find(|arena| arena.cpuid == cpuid)
        .unwrap();
    set_thread_pointer(arena.arena);
}
