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
pub struct Reg<'blob> {
    address_cells: AddressCells,
    size_cells: SizeCells,
    value: &'blob [[u8; 4]],
}

impl<'blob> IntoIterator for Reg<'blob> {
    type Item = iter::RegValue<'blob>;
    type IntoIter = iter::RegIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        iter::RegIter::new(self)
    }
}

impl fmt::Debug for Reg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

impl<'blob> Reg<'blob> {
    /// # Panics
    ///
    /// Panics if `value.len()` is not a multiple of `address_cells +
    /// size_cells`.
    #[must_use]
    pub fn new(
        address_cells: AddressCells,
        size_cells: SizeCells,
        value: &'blob [[u8; 4]],
    ) -> Self {
        assert!(
            value
                .len()
                .is_multiple_of(address_cells.value() + size_cells.value())
        );
        Self {
            address_cells,
            size_cells,
            value,
        }
    }
}

#[derive(DeserializeNode)]
#[devtree(crate = crate)]
struct RegParent {
    #[devtree(property(name = "#address-cells"))]
    address_cells: AddressCells,
    #[devtree(property(name = "#size-cells"))]
    size_cells: SizeCells,
}

impl<'blob> DeserializeProperty<'blob> for Reg<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let value = <&[[u8; 4]]>::deserialize_property(de)?;
        let RegParent {
            address_cells,
            size_cells,
        } = de
            .clone_tree_cursor()?
            .read_parent()
            .ok_or_else(|| DeserializeNodeError::missing_parent_node(de.node()))?
            .deserialize_node()?;

        let unit = address_cells.value() + size_cells.value();
        ensure!(
            value.len().is_multiple_of(unit),
            DeserializePropertyError::value_length_is_not_multiple_of(de.property(), unit)
        );
        Ok(Self::new(address_cells, size_cells, value))
    }
}

pub(crate) mod iter {
    use core::{iter::FusedIterator, ops::Range};

    use platform_cast::CastFrom as _;

    use super::Reg;
    use crate::model::property::{AddressCells, SizeCells, U32Array};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RegValue<'blob> {
        pub address: &'blob U32Array,
        pub size: &'blob U32Array,
    }

    impl RegValue<'_> {
        #[must_use]
        pub fn range(&self) -> Range<usize> {
            let address = self
                .address
                .iter()
                .fold(0, |acc, x| (acc << 32) | usize::cast_from(x));
            let size = self
                .size
                .iter()
                .fold(0, |acc, x| (acc << 32) | usize::cast_from(x));
            address..address.saturating_add(size)
        }
    }

    #[derive(Debug, Clone)]
    pub struct RegIter<'blob> {
        address_cells: AddressCells,
        size_cells: SizeCells,
        value: &'blob [[u8; 4]],
    }

    impl<'blob> RegIter<'blob> {
        pub(crate) fn new(reg: Reg<'blob>) -> Self {
            Self {
                address_cells: reg.address_cells,
                size_cells: reg.size_cells,
                value: reg.value,
            }
        }
    }

    impl<'blob> Iterator for RegIter<'blob> {
        type Item = RegValue<'blob>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.value.is_empty() {
                return None;
            }

            let value = self.value;
            let (address, value) = value.split_at(self.address_cells.value());
            let (size, value) = value.split_at(self.size_cells.value());
            self.value = value;
            let address = U32Array::new(address);
            let size = U32Array::new(size);

            Some(RegValue { address, size })
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = self.value.len() / (self.address_cells.value() + self.size_cells.value());
            (len, Some(len))
        }
    }

    impl DoubleEndedIterator for RegIter<'_> {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.value.is_empty() {
                return None;
            }

            let value = self.value;
            let (value, size) = value.split_at(value.len() - self.size_cells.value());
            let (value, address) = value.split_at(value.len() - self.address_cells.value());
            self.value = value;
            let address = U32Array::new(address);
            let size = U32Array::new(size);

            Some(RegValue { address, size })
        }
    }

    impl ExactSizeIterator for RegIter<'_> {}
    impl FusedIterator for RegIter<'_> {}
}
