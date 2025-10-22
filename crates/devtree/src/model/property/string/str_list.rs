use core::fmt;

use crate::{
    de::{
        DeserializeProperty, PropertyDeserializer,
        error::{DeserializeError, DeserializePropertyError},
    },
    polyfill,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrList<'blob> {
    value: &'blob str,
}

impl<'blob> StrList<'blob> {
    #[must_use]
    pub fn new(value: &'blob str) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn iter(&self) -> iter::StrListIter<'blob> {
        self.into_iter()
    }
}

impl fmt::Debug for StrList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'blob> DeserializeProperty<'blob> for StrList<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let (bytes, _) = polyfill::slice_rsplit_once(property.value(), |&c| c == 0)
            .ok_or_else(|| DeserializePropertyError::missing_nul_in_string_value(property))?;
        let s = str::from_utf8(bytes)
            .map_err(|source| DeserializePropertyError::invalid_string_value(property, source))?;
        Ok(Self::new(s))
    }
}

impl<'blob> IntoIterator for &StrList<'blob> {
    type Item = &'blob str;
    type IntoIter = iter::StrListIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        iter::StrListIter::new(self.value)
    }
}

pub(crate) mod iter {
    use core::iter::FusedIterator;

    #[derive(Clone)]
    pub struct StrListIter<'blob> {
        remainder: &'blob str,
    }

    impl<'blob> StrListIter<'blob> {
        pub(crate) fn new(value: &'blob str) -> Self {
            Self { remainder: value }
        }
    }

    impl<'blob> Iterator for StrListIter<'blob> {
        type Item = &'blob str;

        fn next(&mut self) -> Option<Self::Item> {
            if self.remainder.is_empty() {
                return None;
            }

            let (s, rest) = self
                .remainder
                .split_once('\0')
                .unwrap_or((self.remainder, ""));
            self.remainder = rest;
            Some(s)
        }
    }

    impl DoubleEndedIterator for StrListIter<'_> {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.remainder.is_empty() {
                return None;
            }

            let (rest, s) = self
                .remainder
                .rsplit_once('\0')
                .unwrap_or(("", self.remainder));
            self.remainder = rest;
            Some(s)
        }
    }

    impl FusedIterator for StrListIter<'_> {}
}
