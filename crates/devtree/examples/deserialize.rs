use devtree::Devicetree;

#[expect(dead_code)]
#[path = "../examples/bindings/standard.rs"]
mod standard;

#[repr(C, align(8))]
struct Bytes<const N: usize>([u8; N]);

fn main() {
    let dtb = &Bytes(*include_bytes!("assets/qemu-virt.dtb"));
    let dt = Devicetree::from_bytes(&dtb.0).unwrap();
    let root_node = dt.read_root_node().unwrap();
    let deserialized = root_node.deserialize_node::<standard::Root>().unwrap();
    println!("{deserialized:#?}");

    let dtb = &Bytes(*include_bytes!("assets/qemu-virt-opensbi.dtb"));
    let dt = Devicetree::from_bytes(&dtb.0).unwrap();
    let root_node = dt.read_root_node().unwrap();
    let deserialized = root_node.deserialize_node::<standard::Root>().unwrap();
    println!("{deserialized:#?}");
}
