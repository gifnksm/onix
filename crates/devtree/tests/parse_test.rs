#![cfg(test)]

use devtree::Devicetree;

#[repr(C, align(8))]
struct Bytes<const N: usize>([u8; N]);

#[expect(dead_code)]
#[path = "../examples/bindings/standard.rs"]
mod standard;

#[test]
fn parse_qemu_virt() {
    let dtb = &Bytes(*include_bytes!("../examples/assets/qemu-virt.dtb"));
    let dt = Devicetree::from_bytes(&dtb.0).unwrap();
    let root_node = dt.read_root_node().unwrap();
    let _root = root_node.deserialize_node::<standard::Root>().unwrap();
}

#[test]
fn parse_qemu_virt_opensbi() {
    let dtb = &Bytes(*include_bytes!("../examples/assets/qemu-virt-opensbi.dtb"));
    let dt = Devicetree::from_bytes(&dtb.0).unwrap();
    let root_node = dt.read_root_node().unwrap();
    let _root = root_node.deserialize_node::<standard::Root>().unwrap();
}
