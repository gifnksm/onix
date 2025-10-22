//! The following example demonstrates how to implement the key requirements
//! defined in section *3. Device Node Requirements* of [the Devicetree
//! Specification v0.4]. It also shows how to use various features of the
//! derive macro, such as handling properties, child nodes, defaults, and
//! fallbacks.
//!
//! This example is illustrative and does not cover every devicetree
//! schema detail. Instead, it highlights the most common patterns.
//!
//! [the Devicetree Specification v0.4]: https://github.com/devicetree-org/devicetree-specification/releases/tag/v0.4

use std::collections::BTreeMap;

use devtree::{
    DeserializeNode,
    de::util,
    model::{
        node::{NodeFullName, NodeUnitAddress},
        property::{AddressCells, Compatible, Model, PropertyName, Ranges, Reg, SizeCells, Status},
    },
    types::ByteStr,
};

// Root node demonstrating top-level properties and child nodes.
#[derive(Debug, DeserializeNode)]
pub struct Root<'blob> {
    // Required properties.
    #[devtree(property = "#address-cells")]
    pub address_cells: AddressCells,
    #[devtree(property = "#size-cells")]
    pub size_cells: SizeCells,
    #[devtree(property)]
    pub model: Model<'blob>,
    #[devtree(property)]
    pub compatible: Compatible<'blob>,

    // Optional properties with defaults.
    #[devtree(property(name = "serial-number", default))]
    pub serial_number: Option<&'blob ByteStr>,
    #[devtree(property(name = "chassis-type", default))]
    pub chassis_type: Option<&'blob ByteStr>,

    // Child nodes demonstrating different attribute forms:
    // - aliases: custom deserialization into a PropertyCollection (BTreeMap)
    #[devtree(child(
        default,
        deserialize_with = util::deserialize_node_as_property_collection,
    ))]
    pub aliases: BTreeMap<PropertyName<'blob>, &'blob ByteStr>,

    // - memory: repeated children nodes
    #[devtree(repeated_children)]
    pub memory: Vec<Memory<'blob>>,

    // - reserved_memory: optional child node with default
    #[devtree(child(name = "reserved-memory", default))]
    pub reserved_memory: Option<ReservedMemory<'blob>>,

    // - chosen: optional child node with default
    #[devtree(child(default))]
    pub chosen: Option<Chosen<'blob>>,

    // - cpus: required child node
    #[devtree(child)]
    pub cpus: Cpus<'blob>,
}

// Memory node demonstrating unit address and required properties.
#[derive(Debug, DeserializeNode)]
pub struct Memory<'blob> {
    // Node unit address.
    #[devtree(node)]
    pub unit_address: NodeUnitAddress<'blob>,

    // Required properties.
    #[devtree(property)]
    pub device_type: &'blob ByteStr,
    #[devtree(property)]
    pub reg: Reg<'blob>,
}

// ReservedMemory node demonstrating extra_children to collect unmatched
// children.
#[derive(Debug, DeserializeNode)]
pub struct ReservedMemory<'blob> {
    // Required properties.
    #[devtree(property = "#address-cells")]
    pub address_cells: AddressCells,
    #[devtree(property = "#size-cells")]
    pub size_cells: SizeCells,
    #[devtree(property)]
    pub ranges: Ranges<'blob>,

    // Extra children collected in a BTreeMap.
    #[devtree(extra_children)]
    pub children: BTreeMap<NodeFullName<'blob>, ReservedMemoryChild<'blob>>,
}

// Child nodes under "reserved-memory".
#[derive(Debug, DeserializeNode)]
pub struct ReservedMemoryChild<'blob> {
    // Node full name.
    #[devtree(node)]
    pub full_name: NodeFullName<'blob>,

    // Optional properties with defaults.
    #[devtree(property(default))]
    pub reg: Option<Reg<'blob>>,
}

// Optional chosen node demonstrating default properties.
#[derive(Debug, DeserializeNode)]
pub struct Chosen<'blob> {
    // Optional properties with defaults.
    #[devtree(property(default))]
    pub bootargs: Option<&'blob ByteStr>,
    #[devtree(property(name = "stdout-path", default))]
    pub stdout_path: Option<&'blob ByteStr>,
    #[devtree(property(name = "stdin-path", default))]
    pub stdin_path: Option<&'blob ByteStr>,
}

// CPUs node demonstrating repeated children.
#[derive(Debug, DeserializeNode)]
pub struct Cpus<'blob> {
    // Repeated child nodes.
    #[devtree(property = "#address-cells")]
    pub address_cells: AddressCells,
    #[devtree(property = "#size-cells")]
    pub size_cells: SizeCells,

    // Repeated child nodes with the same name.
    #[devtree(repeated_children)]
    pub cpu: Vec<Cpu<'blob>>,
}

// CPU node demonstrating fallback, default, and custom deserialization.
#[derive(Debug, DeserializeNode)]
pub struct Cpu<'blob> {
    // Node unit address.
    #[devtree(node)]
    pub unit_address: NodeUnitAddress<'blob>,
    // Required properties.
    #[devtree(property)]
    pub device_type: &'blob ByteStr,
    #[devtree(property)]
    pub reg: Reg<'blob>,

    // Optional properties with fallback, default, and custom deserialization.
    #[devtree(property)]
    pub compatible: Compatible<'blob>,
    #[devtree(property(
        name = "clock-frequency",
        fallback = "parent",
        default,
        deserialize_with = |de| util::deserialize_u64_or_u32_property(de).map(Some),
    ))]
    pub clock_frequency: Option<u64>,
    #[devtree(property(
        name = "timebase-frequency",
        fallback = "parent",
        default,
        deserialize_with = |de| util::deserialize_u64_or_u32_property(de).map(Some),
    ))]
    pub timebase_frequency: Option<u64>,

    // Optional property with default.
    #[devtree(property(default))]
    pub status: Status,
}
