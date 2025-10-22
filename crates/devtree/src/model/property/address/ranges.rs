use core::fmt;

use devtree_derive::DeserializeNode;

use crate::{
    de::{
        DeserializeProperty, PropertyDeserializer,
        error::{DeserializeError, DeserializeNodeError, DeserializePropertyError},
    },
    model::property::{AddressCells, SizeCells},
    tree_cursor::TreeCursor as _,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ranges<'blob> {
    child_address_cells: AddressCells,
    child_size_cells: SizeCells,
    parent_address_cells: AddressCells,
    value: &'blob [[u8; 4]],
}

impl<'blob> IntoIterator for Ranges<'blob> {
    type Item = iter::RangesValue<'blob>;
    type IntoIter = iter::RangesIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        iter::RangesIter::new(self)
    }
}

impl fmt::Debug for Ranges<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

impl<'blob> Ranges<'blob> {
    /// # Panics
    ///
    /// Panics if `value.len()` is not a multiple of `child_address_cells +
    /// child_size_cells + parent_address_cells`.
    #[must_use]
    pub fn new(
        child_address_cells: AddressCells,
        child_size_cells: SizeCells,
        parent_address_cells: AddressCells,
        value: &'blob [[u8; 4]],
    ) -> Self {
        assert!(value.len().is_multiple_of(
            child_address_cells.value() + child_size_cells.value() + parent_address_cells.value()
        ));
        Self {
            child_address_cells,
            child_size_cells,
            parent_address_cells,
            value,
        }
    }
}

#[derive(DeserializeNode)]
#[devtree(crate = crate)]
struct RangesParent {
    #[devtree(property(name = "#address-cells"))]
    parent_address_cells: AddressCells,
}

#[derive(DeserializeNode)]
#[devtree(crate = crate)]
struct RangesNode {
    #[devtree(property(name = "#address-cells"))]
    child_address_cells: AddressCells,
    #[devtree(property(name = "#size-cells"))]
    child_size_cells: SizeCells,
}

impl<'blob> DeserializeProperty<'blob> for Ranges<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let RangesParent {
            parent_address_cells,
        } = de
            .clone_tree_cursor()?
            .read_parent()
            .ok_or_else(|| DeserializeNodeError::missing_parent_node(de.node()))?
            .deserialize_node()?;
        let RangesNode {
            child_address_cells,
            child_size_cells,
        } = de.clone_tree_cursor()?.read_node().deserialize_node()?;

        let value = <&[[u8; 4]]>::deserialize_property(de)?;

        let unit =
            parent_address_cells.value() + child_address_cells.value() + child_size_cells.value();
        ensure!(
            value.len().is_multiple_of(unit),
            DeserializePropertyError::value_length_is_not_multiple_of(de.property(), unit,)
        );
        Ok(Self::new(
            child_address_cells,
            child_size_cells,
            parent_address_cells,
            value,
        ))
    }
}

pub(crate) mod iter {
    use core::iter::FusedIterator;

    use super::Ranges;
    use crate::model::property::{AddressCells, SizeCells, U32Array};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RangesValue<'blob> {
        pub child_bus_address: &'blob U32Array,
        pub parent_bus_address: &'blob U32Array,
        pub len: &'blob U32Array,
    }

    #[derive(Debug, Clone)]
    pub struct RangesIter<'blob> {
        child_address_cells: AddressCells,
        child_size_cells: SizeCells,
        parent_address_cells: AddressCells,
        value: &'blob [[u8; 4]],
    }

    impl<'blob> RangesIter<'blob> {
        pub(crate) fn new(ranges: Ranges<'blob>) -> Self {
            Self {
                child_address_cells: ranges.child_address_cells,
                child_size_cells: ranges.child_size_cells,
                parent_address_cells: ranges.parent_address_cells,
                value: ranges.value,
            }
        }
    }

    impl<'blob> Iterator for RangesIter<'blob> {
        type Item = RangesValue<'blob>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.value.is_empty() {
                return None;
            }

            let value = self.value;
            let (child_bus_address, value) = value.split_at(self.child_address_cells.value());
            let (parent_bus_address, value) = value.split_at(self.parent_address_cells.value());
            let (len, value) = value.split_at(self.child_size_cells.value());
            self.value = value;
            let child_bus_address = U32Array::new(child_bus_address);
            let parent_bus_address = U32Array::new(parent_bus_address);
            let len = U32Array::new(len);

            Some(RangesValue {
                child_bus_address,
                parent_bus_address,
                len,
            })
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = self.value.len()
                / (self.child_address_cells.value()
                    + self.child_size_cells.value()
                    + self.parent_address_cells.value());
            (len, Some(len))
        }
    }

    impl DoubleEndedIterator for RangesIter<'_> {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.value.is_empty() {
                return None;
            }

            let value = self.value;
            let (value, len) = value.split_at(value.len() - self.child_size_cells.value());
            let (value, parent_bus_address) =
                value.split_at(value.len() - self.parent_address_cells.value());
            let (value, child_bus_address) =
                value.split_at(value.len() - self.child_address_cells.value());
            self.value = value;
            let child_bus_address = U32Array::new(child_bus_address);
            let parent_bus_address = U32Array::new(parent_bus_address);
            let len = U32Array::new(len);

            Some(RangesValue {
                child_bus_address,
                parent_bus_address,
                len,
            })
        }
    }

    impl ExactSizeIterator for RangesIter<'_> {}
    impl FusedIterator for RangesIter<'_> {}
}
