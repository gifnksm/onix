#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
#![cfg(test)]

use devtree::{Devicetree, tree_cursor::TreeCursor as _};

#[repr(C, align(8))]
struct Bytes<const N: usize>([u8; N]);

#[expect(dead_code)]
#[path = "../examples/bindings/standard.rs"]
mod standard;

#[test]
fn parse_qemu_virt() {
    let dtb = &Bytes(*include_bytes!("../examples/assets/qemu-virt.dtb"));
    let dt = Devicetree::from_bytes(&dtb.0).unwrap();
    let _root = dt
        .tree_cursor()
        .unwrap()
        .read_node()
        .deserialize_node::<standard::Root>()
        .unwrap();
}

#[test]
fn parse_qemu_virt_opensbi() {
    let dtb = &Bytes(*include_bytes!("../examples/assets/qemu-virt-opensbi.dtb"));
    let dt = Devicetree::from_bytes(&dtb.0).unwrap();
    let _root = dt
        .tree_cursor()
        .unwrap()
        .read_node()
        .deserialize_node::<standard::Root>()
        .unwrap();
}
