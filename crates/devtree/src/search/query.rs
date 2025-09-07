use core::ptr;

use crate::{
    blob::{Node, PATH_SEPARATOR, UNIT_ADDRESS_SEPARATOR},
    types::ByteStr,
};

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct NodeQuery {
    value: ByteStr,
}

impl NodeQuery {
    #[must_use]
    pub fn new<Q>(query: &Q) -> &Self
    where
        Q: AsRef<ByteStr> + ?Sized,
    {
        let query = query.as_ref();
        // SAFETY: Query is #[repr(transparent)] over ByteStr
        #[expect(clippy::missing_panics_doc)]
        unsafe {
            (ptr::from_ref(query) as *const Self).as_ref().unwrap()
        }
    }

    #[must_use]
    pub fn is_absolute(&self) -> bool {
        self.value.starts_with(&[PATH_SEPARATOR])
    }

    #[must_use]
    pub fn value(&self) -> &ByteStr {
        &self.value
    }

    #[must_use]
    pub fn split_first_component(&self) -> Option<(QueryComponent<'_>, &Self)> {
        let value = &self.value;

        let n = next_non_separator(value).unwrap_or(value.len());
        let value = &value[n..];
        if value.is_empty() {
            return None;
        }

        let n = next_separator(value).unwrap_or(value.len());
        let component = &value[..n];
        let rest = &value[n..];

        let component = if component == b"*" {
            QueryComponent::Wildcard
        } else if component.contains(&UNIT_ADDRESS_SEPARATOR) {
            QueryComponent::FullName(component)
        } else {
            QueryComponent::Name(component)
        };

        let n = next_non_separator(rest).unwrap_or(rest.len());
        let rest = &rest[n..];
        Some((component, Self::new(rest)))
    }
}

impl<T> PartialEq<T> for NodeQuery
where
    ByteStr: PartialEq<T>,
{
    fn eq(&self, other: &T) -> bool {
        &self.value == other
    }
}

impl AsRef<Self> for NodeQuery {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<NodeQuery> for ByteStr {
    fn as_ref(&self) -> &NodeQuery {
        NodeQuery::new(self)
    }
}

impl AsRef<NodeQuery> for &str {
    fn as_ref(&self) -> &NodeQuery {
        NodeQuery::new(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryComponent<'q> {
    Wildcard,
    Name(&'q ByteStr),
    FullName(&'q ByteStr),
}

impl QueryComponent<'_> {
    pub(crate) fn match_node(&self, node: &Node<'_>) -> bool {
        match self {
            Self::Wildcard => true,
            Self::Name(name) => node.name() == *name,
            Self::FullName(full_name) => node.full_name() == *full_name,
        }
    }
}

fn next_non_separator(value: &[u8]) -> Option<usize> {
    value.iter().position(|&b| b != PATH_SEPARATOR)
}

fn next_separator(value: &[u8]) -> Option<usize> {
    value.iter().position(|&b| b == PATH_SEPARATOR)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bs(s: &str) -> &ByteStr {
        ByteStr::new(s.as_bytes())
    }

    fn name(s: &str) -> QueryComponent<'_> {
        QueryComponent::Name(bs(s))
    }

    fn full_name(s: &str) -> QueryComponent<'_> {
        QueryComponent::FullName(bs(s))
    }

    #[track_caller]
    fn check_components(query: &str, components: &[QueryComponent]) {
        let mut query = NodeQuery::new(query);
        for (i, expected_component) in components.iter().enumerate() {
            let (component, rest) = query.split_first_component().unwrap();
            assert_eq!(*expected_component, component, "{i}");
            assert!(!rest.is_absolute(), "{i}");
            query = rest;
        }
        assert!(query.split_first_component().is_none());
    }

    #[test]
    fn test_query_new() {
        let q = "foo/bar@1";
        let query = NodeQuery::new(q);
        assert_eq!(&query.value, q);
    }

    #[test]
    fn test_query_is_absolute() {
        let query = NodeQuery::new("/foo/bar@1");
        assert!(query.is_absolute());

        let query = NodeQuery::new("foo/bar@1");
        assert!(!query.is_absolute());
    }

    #[test]
    fn test_relative_query() {
        check_components(
            "foo/bar@1/baz",
            &[name("foo"), full_name("bar@1"), name("baz")],
        );
        check_components("a/b/c", &[name("a"), name("b"), name("c")]);
        check_components("a", &[name("a")]);
    }

    #[test]
    fn test_absolute_query() {
        check_components(
            "/foo/bar@1/baz",
            &[name("foo"), full_name("bar@1"), name("baz")],
        );
        check_components("/a/b/c", &[name("a"), name("b"), name("c")]);
        check_components("/a", &[name("a")]);
    }

    #[test]
    fn test_empty_query() {
        check_components("", &[]);
        check_components("/", &[]);
    }

    #[test]
    fn test_query_with_only_separators() {
        check_components("///", &[]);
    }

    #[test]
    fn test_consecutive_separators() {
        check_components(
            "/foo//bar///baz//",
            &[name("foo"), name("bar"), name("baz")],
        );
        check_components("foo//bar///baz//", &[name("foo"), name("bar"), name("baz")]);
    }
}
