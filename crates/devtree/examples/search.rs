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
    root_node
        .visit_all_nodes_by_query("/memory", |node| {
            println!("{node:#?}");
        })
        .unwrap();

    let dtb = &Bytes(*include_bytes!("assets/qemu-virt-opensbi.dtb"));
    let dt = Devicetree::from_bytes(&dtb.0).unwrap();
    let root_node = dt.read_root_node().unwrap();
    root_node
        .visit_all_nodes_by_query("/memory", |node| {
            println!("{node:#?}");
        })
        .unwrap();
}
