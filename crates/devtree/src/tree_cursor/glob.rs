use core::ptr;

use crate::{
    blob::{Node, PATH_SEPARATOR, UNIT_ADDRESS_SEPARATOR},
    types::ByteStr,
};

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Glob {
    value: ByteStr,
}

impl Glob {
    #[must_use]
    pub fn new<G>(glob: &G) -> &Self
    where
        G: AsRef<ByteStr> + ?Sized,
    {
        let glob = glob.as_ref();
        // SAFETY: Glob is #[repr(transparent)] over ByteStr
        #[expect(clippy::missing_panics_doc)]
        unsafe {
            (ptr::from_ref(glob) as *const Self).as_ref().unwrap()
        }
    }

    #[must_use]
    pub fn is_absolute(&self) -> bool {
        self.value.starts_with(&[PATH_SEPARATOR])
    }

    #[must_use]
    pub fn as_byte_str(&self) -> &ByteStr {
        &self.value
    }

    #[must_use]
    pub fn cursor(&self) -> GlobCursor<'_> {
        GlobCursor::new(self)
    }
}

impl<T> PartialEq<T> for Glob
where
    ByteStr: PartialEq<T>,
{
    fn eq(&self, other: &T) -> bool {
        &self.value == other
    }
}

impl AsRef<Self> for Glob {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<Glob> for ByteStr {
    fn as_ref(&self) -> &Glob {
        Glob::new(self)
    }
}

impl AsRef<Glob> for str {
    fn as_ref(&self) -> &Glob {
        Glob::new(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::IsVariant)]
pub enum GlobComponent<'glob> {
    RootNode,
    Wildcard,
    Name(&'glob ByteStr),
    FullName(&'glob ByteStr),
}

impl GlobComponent<'_> {
    #[must_use]
    pub fn match_node(&self, node: &Node<'_>) -> bool {
        match self {
            GlobComponent::RootNode => node.is_root(),
            GlobComponent::Wildcard => true,
            GlobComponent::Name(name) => node.name() == *name,
            GlobComponent::FullName(full_name) => node.full_name() == *full_name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorState {
    Top,
    RootNode,
    Component { start: usize, end: usize },
    Bottom,
}

#[derive(Debug)]
pub struct GlobCursor<'glob> {
    glob: &'glob Glob,
    state: CursorState,
}

impl<'glob> GlobCursor<'glob> {
    #[must_use]
    pub fn new<Q>(glob: &'glob Q) -> Self
    where
        Q: AsRef<Glob> + ?Sized + 'glob,
    {
        let glob = glob.as_ref();
        let mut this = Self {
            glob,
            state: CursorState::Top,
        };
        this.seek_descend();
        this
    }

    #[must_use]
    pub fn current_component(&self) -> Option<GlobComponent<'glob>> {
        match self.state {
            CursorState::Top | CursorState::Bottom => None,
            CursorState::RootNode => Some(GlobComponent::RootNode),
            CursorState::Component { start, end } => {
                let component = ByteStr::new(&self.glob.as_byte_str()[start..end]);
                if component == b"*" {
                    Some(GlobComponent::Wildcard)
                } else if component.contains(&UNIT_ADDRESS_SEPARATOR) {
                    Some(GlobComponent::FullName(component))
                } else {
                    Some(GlobComponent::Name(component))
                }
            }
        }
    }

    pub fn seek_descend(&mut self) -> Option<()> {
        let glob_bytes = &self.glob.as_byte_str();
        let prev_end = match self.state {
            CursorState::Top if self.glob.is_absolute() => {
                self.state = CursorState::RootNode;
                return Some(());
            }
            CursorState::Top | CursorState::RootNode => 0,
            CursorState::Component { end, .. } => end,
            CursorState::Bottom => return None,
        };

        if let Some(start) = next_non_separator(glob_bytes, prev_end) {
            let end = next_separator(glob_bytes, start).unwrap_or(glob_bytes.len());
            self.state = CursorState::Component { start, end };
        } else {
            self.state = CursorState::Bottom;
        }

        Some(())
    }

    pub fn seek_ascend(&mut self) -> Option<()> {
        let glob_bytes = &self.glob.as_byte_str();
        let next_start = match self.state {
            CursorState::Top => {
                return None;
            }
            CursorState::RootNode => {
                self.state = CursorState::Top;
                return Some(());
            }
            CursorState::Component { start, .. } => start,
            CursorState::Bottom => glob_bytes.len(),
        };

        if let Some(max) = prev_non_separator(glob_bytes, next_start) {
            let start = prev_separator(glob_bytes, max).map_or(0, |n| n + 1);
            self.state = CursorState::Component {
                start,
                end: max + 1,
            }
        } else if self.glob.is_absolute() {
            self.state = CursorState::RootNode;
        } else {
            self.state = CursorState::Top;
        }

        Some(())
    }
}

fn next_non_separator(value: &[u8], start: usize) -> Option<usize> {
    value[start..]
        .iter()
        .position(|&b| b != PATH_SEPARATOR)
        .map(|n| n + start)
}

fn next_separator(value: &[u8], start: usize) -> Option<usize> {
    value[start..]
        .iter()
        .position(|&b| b == PATH_SEPARATOR)
        .map(|n| n + start)
}

fn prev_non_separator(value: &[u8], end: usize) -> Option<usize> {
    value[..end].iter().rposition(|&b| b != PATH_SEPARATOR)
}

fn prev_separator(value: &[u8], end: usize) -> Option<usize> {
    value[..end].iter().rposition(|&b| b == PATH_SEPARATOR)
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;

    use super::*;

    fn bs(s: &str) -> &ByteStr {
        ByteStr::new(s.as_bytes())
    }

    fn root() -> GlobComponent<'static> {
        GlobComponent::RootNode
    }

    fn name(s: &str) -> GlobComponent<'_> {
        GlobComponent::Name(bs(s))
    }

    fn full_name(s: &str) -> GlobComponent<'_> {
        GlobComponent::FullName(bs(s))
    }

    #[track_caller]
    fn check_components(glob: &str, expected_components: &[GlobComponent]) {
        let glob = Glob::new(glob);
        let mut cursor = glob.cursor();
        let mut components = vec![];
        while let Some(comp) = cursor.current_component() {
            components.push(comp);
            assert!(cursor.seek_descend().is_some());
        }
        assert!(cursor.seek_descend().is_none());
        assert_eq!(components, expected_components);

        assert!(cursor.seek_ascend().is_some());
        let mut components = vec![];
        while let Some(comp) = cursor.current_component() {
            components.push(comp);
            assert!(cursor.seek_ascend().is_some());
        }
        assert!(cursor.seek_ascend().is_none());
        components.reverse();
        assert_eq!(components, expected_components);
    }

    #[test]
    fn test_glob_new() {
        let g = "foo/bar@1";
        let glob = Glob::new(g);
        assert_eq!(&glob.value, g);
    }

    #[test]
    fn test_glob_is_absolute() {
        let glob = Glob::new("/foo/bar@1");
        assert!(glob.is_absolute());

        let glob = Glob::new("foo/bar@1");
        assert!(!glob.is_absolute());
    }

    #[test]
    fn test_relative_glob() {
        check_components(
            "foo/bar@1/baz",
            &[name("foo"), full_name("bar@1"), name("baz")],
        );
        check_components("a/b/c", &[name("a"), name("b"), name("c")]);
        check_components("a", &[name("a")]);
    }

    #[test]
    fn test_absolute_glob() {
        check_components(
            "/foo/bar@1/baz",
            &[root(), name("foo"), full_name("bar@1"), name("baz")],
        );
        check_components("/a/b/c", &[root(), name("a"), name("b"), name("c")]);
        check_components("/a", &[root(), name("a")]);
    }

    #[test]
    fn test_empty_glob() {
        check_components("", &[]);
    }

    #[test]
    fn test_glob_with_only_separators() {
        check_components("///", &[root()]);
    }

    #[test]
    fn test_consecutive_separators() {
        check_components(
            "/foo//bar///baz//",
            &[root(), name("foo"), name("bar"), name("baz")],
        );
        check_components("foo//bar///baz//", &[name("foo"), name("bar"), name("baz")]);
    }

    #[test]
    fn test_wildcard_component() {
        check_components(
            "/foo/*/bar/*/baz",
            &[
                root(),
                name("foo"),
                GlobComponent::Wildcard,
                name("bar"),
                GlobComponent::Wildcard,
                name("baz"),
            ],
        );
        check_components(
            "foo/*/bar/*/baz",
            &[
                name("foo"),
                GlobComponent::Wildcard,
                name("bar"),
                GlobComponent::Wildcard,
                name("baz"),
            ],
        );
        check_components(
            "*/foo/*/bar/*/baz/*",
            &[
                GlobComponent::Wildcard,
                name("foo"),
                GlobComponent::Wildcard,
                name("bar"),
                GlobComponent::Wildcard,
                name("baz"),
                GlobComponent::Wildcard,
            ],
        );
        check_components("*", &[GlobComponent::Wildcard]);
        check_components("/*", &[root(), GlobComponent::Wildcard]);
        check_components("//*", &[root(), GlobComponent::Wildcard]);
        check_components("*//", &[GlobComponent::Wildcard]);
    }
}
