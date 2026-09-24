cfg_select! {
    target_arch = "riscv64" => {
        mod riscv64;
        pub use riscv64::*;
    }
    _ => {
        mod unsupported;
        pub use unsupported::*;
    }
}
