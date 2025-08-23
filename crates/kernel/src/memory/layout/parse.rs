use core::{ops::Range, ptr};

use devicetree::{
    common::property::{ParsePropertyValue, ParsePropertyValueError, Property, RegIter},
    flattened::{
        Devicetree,
        node::{Node, ParseStructError},
    },
};
use platform_cast::CastInto as _;
use range_set::RangeSet;
use snafu::{ResultExt as _, Snafu};
use snafu_utils::Location;

pub fn insert_memory_ranges(
    dtb: &Devicetree,
    ranges: &mut RangeSet<128>,
) -> Result<(), DevicetreeError> {
    let parent = dtb.root_node().context(ParseStructSnafu)?;
    let address_cells = find_prop_address_cells(&parent)?;
    let size_cells = find_prop_size_cells(&parent)?;

    for child in parent.children() {
        let child = child.context(ParseStructSnafu)?;
        if child.name() != "memory" {
            continue;
        }
        let reg_iter = find_prop_reg(&child, address_cells, size_cells)?;
        for reg in reg_iter {
            ranges.insert(reg.range());
        }
    }

    Ok(())
}

pub fn remove_reserved_ranges(
    dtb: &Devicetree,
    ranges: &mut RangeSet<128>,
) -> Result<(), DevicetreeError> {
    for rsv in dtb.mem_rsvmap() {
        ranges.remove(rsv.range());
    }

    let root = dtb.root_node().context(ParseStructSnafu)?;
    if let Some(parent) = find_child(&root, "reserved-memory")? {
        let address_cells = find_prop_address_cells(&parent)?;
        let size_cells = find_prop_size_cells(&parent)?;
        for child in parent.children() {
            let child = child.context(ParseStructSnafu)?;
            let reg_iter = find_prop_reg(&child, address_cells, size_cells)?;
            for reg in reg_iter {
                ranges.remove(reg.range());
            }
        }
    }

    Ok(())
}

pub fn dtb_range(dtb: &Devicetree<'_>) -> Range<usize> {
    let header = dtb.header();
    let dtb_start = ptr::from_ref(header).addr();
    let dtb_end = dtb_start + dtb.size();
    super::super::expand_to_page_boundaries(dtb_start..dtb_end)
}

#[derive(Debug, Snafu)]
pub enum DevicetreeError {
    #[snafu(display("invalid struct: {source}"))]
    ParseStruct {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ParseStructError,
    },
    #[snafu(display("missing `{name}` property"))]
    MissingProperty {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid property `{name}`: {source}"))]
    ParsePropertyValue {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ParsePropertyValueError,
    },
}

fn find_child<'fdt, 'tree>(
    node: &'tree Node<'fdt, 'tree>,
    name: &str,
) -> Result<Option<Node<'fdt, 'tree>>, DevicetreeError> {
    for child in node.children() {
        let child = child.context(ParseStructSnafu)?;
        if child.name() == name {
            return Ok(Some(child));
        }
    }
    Ok(None)
}

fn find_prop<'fdt>(
    node: &Node<'fdt, '_>,
    name: &'static str,
) -> Result<Property<'fdt>, DevicetreeError> {
    for prop in node.properties() {
        let prop = prop.context(ParseStructSnafu)?;
        if prop.name() == name {
            return Ok(prop);
        }
    }
    Err(MissingPropertySnafu { name }.build())
}

fn find_prop_as<'fdt, T>(node: &Node<'fdt, '_>, name: &'static str) -> Result<T, DevicetreeError>
where
    T: ParsePropertyValue<'fdt>,
{
    let prop = find_prop(node, name)?;
    prop.parse_value().context(ParsePropertyValueSnafu { name })
}

fn find_prop_address_cells(node: &Node<'_, '_>) -> Result<usize, DevicetreeError> {
    find_prop_as(node, "#address-cells").map(u32::cast_into)
}

fn find_prop_size_cells(node: &Node<'_, '_>) -> Result<usize, DevicetreeError> {
    find_prop_as(node, "#size-cells").map(u32::cast_into)
}

fn find_prop_reg<'fdt>(
    node: &Node<'fdt, '_>,
    address_cells: usize,
    size_cells: usize,
) -> Result<RegIter<'fdt>, DevicetreeError> {
    let name = "reg";
    let prop = find_prop(node, name)?;
    prop.parse_value_as_reg(address_cells, size_cells)
        .context(ParsePropertyValueSnafu { name })
}
