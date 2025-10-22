use core::fmt;

use crate::{
    de::{
        DeserializeProperty, PropertyDeserializer,
        error::{DeserializeError, DeserializePropertyError},
    },
    polyfill,
    types::ByteStr,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteStrList<'blob> {
    value: &'blob ByteStr,
}

impl<'blob> ByteStrList<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn iter(&self) -> iter::ByteStrListIter<'blob> {
        self.into_iter()
    }
}

impl fmt::Debug for ByteStrList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'blob> DeserializeProperty<'blob> for ByteStrList<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let (bytes, _) = polyfill::slice_rsplit_once(property.value(), |&c| c == 0)
            .ok_or_else(|| DeserializePropertyError::missing_nul_in_string_value(property))?;
        Ok(Self::new(ByteStr::new(bytes)))
    }
}

impl<'blob> IntoIterator for &ByteStrList<'blob> {
    type Item = &'blob ByteStr;
    type IntoIter = iter::ByteStrListIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        iter::ByteStrListIter::new(self.value)
    }
}

pub(crate) mod iter {
    use core::iter::FusedIterator;

    use crate::{polyfill, types::ByteStr};

    #[derive(Clone)]
    pub struct ByteStrListIter<'blob> {
        remainder: &'blob ByteStr,
    }

    impl<'blob> ByteStrListIter<'blob> {
        pub(crate) fn new(value: &'blob ByteStr) -> Self {
            Self { remainder: value }
        }
    }

    impl<'blob> Iterator for ByteStrListIter<'blob> {
        type Item = &'blob ByteStr;

        fn next(&mut self) -> Option<Self::Item> {
            if self.remainder.is_empty() {
                return None;
            }
            let (s, rest) = polyfill::slice_split_once(self.remainder, |&b| b == 0)
                .unwrap_or((self.remainder, &[]));
            self.remainder = ByteStr::new(rest);
            Some(ByteStr::new(s))
        }
    }

    impl DoubleEndedIterator for ByteStrListIter<'_> {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.remainder.is_empty() {
                return None;
            }
            let (rest, s) = polyfill::slice_rsplit_once(self.remainder, |&b| b == 0)
                .unwrap_or((&[], self.remainder));
            self.remainder = ByteStr::new(rest);
            Some(ByteStr::new(s))
        }
    }

    impl FusedIterator for ByteStrListIter<'_> {}
}
