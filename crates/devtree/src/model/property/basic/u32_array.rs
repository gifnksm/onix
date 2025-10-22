use core::{fmt, ptr};

use crate::de::{DeserializeProperty, PropertyDeserializer, error::DeserializeError};

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct U32Array {
    value: [[u8; 4]],
}

impl U32Array {
    #[must_use]
    pub fn new(value: &[[u8; 4]]) -> &Self {
        // SAFETY: U32Array is #[repr(transparent)] over [[u8; 4]]
        #[expect(clippy::missing_panics_doc)]
        unsafe {
            (ptr::from_ref(value) as *const Self).as_ref().unwrap()
        }
    }

    #[must_use]
    pub fn iter(&self) -> iter::U32ArrayIter<'_> {
        self.into_iter()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.value.len()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<u32> {
        self.value.get(index).copied().map(u32::from_be_bytes)
    }
}

impl<'blob> DeserializeProperty<'blob> for &'blob U32Array {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(U32Array::new)
    }
}

impl<'blob> IntoIterator for &'blob U32Array {
    type Item = u32;
    type IntoIter = iter::U32ArrayIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        iter::U32ArrayIter::new(&self.value)
    }
}

impl fmt::Debug for U32Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

pub(crate) mod iter {
    use core::{iter::FusedIterator, slice};

    #[derive(Debug)]
    pub struct U32ArrayIter<'blob> {
        iter: slice::Iter<'blob, [u8; 4]>,
    }

    impl<'blob> U32ArrayIter<'blob> {
        pub(crate) fn new(value: &'blob [[u8; 4]]) -> Self {
            Self { iter: value.iter() }
        }
    }

    impl Iterator for U32ArrayIter<'_> {
        type Item = u32;

        fn next(&mut self) -> Option<Self::Item> {
            let bytes = self.iter.next()?;
            Some(u32::from_be_bytes(*bytes))
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }

    impl DoubleEndedIterator for U32ArrayIter<'_> {
        fn next_back(&mut self) -> Option<Self::Item> {
            let bytes = self.iter.next_back()?;
            Some(u32::from_be_bytes(*bytes))
        }
    }

    impl FusedIterator for U32ArrayIter<'_> {}
    impl ExactSizeIterator for U32ArrayIter<'_> {}
}
