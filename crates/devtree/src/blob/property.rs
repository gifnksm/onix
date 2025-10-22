use crate::{
    bytes::LazyCStr,
    de::{DeserializeProperty, PropertyDeserializer, error::DeserializeError},
    types::ByteStr,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property<'blob> {
    name: LazyCStr<'blob>,
    value: &'blob [u8],
}

impl<'blob> Property<'blob> {
    #[must_use]
    pub fn new<N, V>(name: &'blob N, value: &'blob V) -> Self
    where
        N: AsRef<[u8]> + ?Sized,
        V: AsRef<[u8]> + ?Sized,
    {
        let name = LazyCStr::new(name);
        let value = value.as_ref();
        Self { name, value }
    }

    #[must_use]
    pub fn name(&self) -> &'blob ByteStr {
        self.name.as_byte_str()
    }

    #[must_use]
    pub fn value(&self) -> &'blob [u8] {
        self.value
    }
}

impl<'blob> DeserializeProperty<'blob> for Property<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(de.property().clone())
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_new_and_accessors() {
        let name = b"test_name";
        let value = b"test_value";
        let prop = Property::new(name, value);

        assert_eq!(prop.name(), name);
        assert_eq!(prop.value(), value);
    }

    #[test]
    fn test_property_name_with_null_separator() {
        let name = b"foo\0bar";
        let value = b"baz";
        let prop = Property::new(name, value);

        assert_eq!(prop.name(), b"foo");
        assert_eq!(prop.value(), value);
    }

    #[test]
    fn test_property_equality() {
        let prop1 = Property::new(b"name", b"value");
        let prop2 = Property::new(b"name", b"value");
        let prop3 = Property::new(b"name", b"other");

        assert_eq!(prop1, prop2);
        assert_ne!(prop1, prop3);
    }
}
