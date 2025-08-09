//! A collection type for storing non-overlapping ranges.
//!
//! `RangeSet` is a data structure that efficiently stores and manages a
//! collection of `Range<usize>` values. It automatically merges overlapping and
//! adjacent ranges, maintaining them in sorted order by their start positions.
//!
//! # Features
//!
//! - **Automatic merging**: Overlapping or adjacent ranges are automatically
//!   merged
//! - **Sorted order**: Ranges are kept sorted by their start positions
//! - **No-std support**: Can be used in `no_std` environments
//! - **Fixed capacity**: Uses `ArrayVec` for stack-allocated storage
//!
//! # Examples
//!
//! ```
//! use range_set::RangeSet;
//!
//! let mut set = RangeSet::<10>::new();
//!
//! // Insert some ranges
//! set.insert(1..5);
//! set.insert(7..10);
//! set.insert(4..8); // This will merge with existing ranges
//!
//! assert_eq!(set.as_slice(), &[1..10]); // All ranges are merged
//!
//! // Remove a portion
//! set.remove(3..6);
//! assert_eq!(set.as_slice(), &[1..3, 6..10]); // Range is split
//! ```
//!
//! # Performance
//!
//! - Insert: O(n) worst case, where n is the number of existing ranges
//! - Remove: O(n) worst case
//! - Iteration: O(1) per element
//! - Memory: Stack-allocated with fixed capacity

#![cfg_attr(not(test), no_std)]

use core::{mem, ops::Range, slice};

use arrayvec::ArrayVec;

/// A collection of non-overlapping ranges stored in sorted order.
///
/// `RangeSet` maintains a collection of `Range<usize>` values that are
/// automatically merged when they overlap or are adjacent. The ranges are kept
/// in sorted order by their start positions.
///
/// # Examples
///
/// ```
/// use range_set::RangeSet;
///
/// let mut set = RangeSet::<10>::new();
/// set.insert(1..5);
/// set.insert(3..7);
/// assert_eq!(set.as_slice(), &[1..7]); // Ranges are merged
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct RangeSet<const CAP: usize> {
    ranges: ArrayVec<Range<usize>, CAP>,
}

impl<const CAP: usize> RangeSet<CAP> {
    /// Creates a new empty `RangeSet`.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let set = RangeSet::<10>::new();
    /// assert!(set.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an iterator over the ranges in the set.
    ///
    /// The ranges are returned in sorted order by their start positions.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let mut set = RangeSet::<10>::new();
    /// set.insert(1..3);
    /// set.insert(5..7);
    ///
    /// let ranges: Vec<_> = set.iter().cloned().collect();
    /// assert_eq!(ranges, vec![1..3, 5..7]);
    /// ```
    pub fn iter(&self) -> slice::Iter<'_, Range<usize>> {
        self.ranges.iter()
    }

    /// Returns a slice containing all ranges in the set.
    ///
    /// The ranges are in sorted order by their start positions.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let mut set = RangeSet::<10>::new();
    /// set.insert(1..3);
    /// set.insert(5..7);
    ///
    /// assert_eq!(set.as_slice(), &[1..3, 5..7]);
    /// ```
    #[must_use]
    pub fn as_slice(&self) -> &[Range<usize>] {
        self.ranges.as_slice()
    }

    /// Returns `true` if the set contains no ranges.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let mut set = RangeSet::<10>::new();
    /// assert!(set.is_empty());
    ///
    /// set.insert(1..3);
    /// assert!(!set.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Inserts a range into the set.
    ///
    /// If the range overlaps with existing ranges or is adjacent to them,
    /// they will be automatically merged into a single range.
    ///
    /// # Panics
    ///
    /// Panics if `insert_range.start > insert_range.end`.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let mut set = RangeSet::<10>::new();
    /// set.insert(1..5);
    /// set.insert(3..7); // Overlaps with existing range
    /// assert_eq!(set.as_slice(), &[1..7]); // Ranges are merged
    /// ```
    pub fn insert(&mut self, insert_range: Range<usize>) {
        assert!(
            insert_range.start <= insert_range.end,
            "Invalid range: {insert_range:?}"
        );
        if insert_range.is_empty() {
            return;
        }

        let mut inserted = false;
        let mut ir = insert_range;
        let mut ranges = mem::take(&mut self.ranges).into_iter();
        for r in ranges.by_ref() {
            if ir.end < r.start {
                inserted = true;
                self.ranges.push(ir.clone());
                self.ranges.push(r);
                break;
            }

            if ir.start > r.end {
                self.ranges.push(r);
                continue;
            }

            ir.start = usize::min(ir.start, r.start);
            ir.end = usize::max(ir.end, r.end);
        }
        if inserted {
            self.ranges.extend(ranges);
        } else {
            assert!(ranges.as_slice().is_empty());
            self.ranges.push(ir);
        }
    }

    /// Removes a range from the set.
    ///
    /// Any existing ranges that overlap with the specified range will be
    /// trimmed or split as necessary. Ranges that don't overlap are left
    /// unchanged.
    ///
    /// # Panics
    ///
    /// Panics if `remove_range.start > remove_range.end`.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let mut set = RangeSet::<10>::new();
    /// set.insert(1..10);
    /// set.remove(3..7);
    /// assert_eq!(set.as_slice(), &[1..3, 7..10]); // Range is split
    /// ```
    pub fn remove(&mut self, remove_range: Range<usize>) {
        assert!(
            remove_range.start <= remove_range.end,
            "Invalid range: {remove_range:?}"
        );
        if remove_range.is_empty() {
            return;
        }

        let rr = remove_range;
        let mut ranges = mem::take(&mut self.ranges).into_iter();
        for r in ranges.by_ref() {
            if rr.end < r.start {
                self.ranges.push(r);
                break;
            }
            if r.start < rr.end && rr.start < r.end {
                if r.start < rr.start {
                    self.ranges.push(r.start..rr.start);
                }
                if rr.end < r.end {
                    self.ranges.push(rr.end..r.end);
                }
            } else {
                self.ranges.push(r);
            }
        }
        self.ranges.extend(ranges);
    }

    /// Returns a new `RangeSet` containing the ranges in this set that are not
    /// in the other set.
    ///
    /// This is equivalent to subtracting all ranges in `other` from this set.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let mut set1 = RangeSet::<10>::new();
    /// set1.insert(1..10);
    ///
    /// let mut set2 = RangeSet::<10>::new();
    /// set2.insert(3..7);
    ///
    /// let diff = set1.difference(&set2);
    /// assert_eq!(diff.as_slice(), &[1..3, 7..10]);
    /// ```
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for r in &other.ranges {
            result.remove(r.clone());
        }
        result
    }
}

impl<const CAP: usize> FromIterator<Range<usize>> for RangeSet<CAP> {
    fn from_iter<T: IntoIterator<Item = Range<usize>>>(iter: T) -> Self {
        let mut this = Self::new();
        this.extend(iter);
        this
    }
}

impl<const CAP: usize> Extend<Range<usize>> for RangeSet<CAP> {
    fn extend<T: IntoIterator<Item = Range<usize>>>(&mut self, iter: T) {
        for range in iter {
            self.ranges.push(range);
        }
    }
}

impl<const CAP: usize> IntoIterator for RangeSet<CAP> {
    type Item = Range<usize>;
    type IntoIter = IntoIter<CAP>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.ranges.into_iter(),
        }
    }
}

/// An iterator that moves out of a `RangeSet`.
///
/// This struct is created by the `into_iter` method on `RangeSet`.
pub struct IntoIter<const CAP: usize> {
    iter: arrayvec::IntoIter<Range<usize>, CAP>,
}

impl<const CAP: usize> IntoIter<CAP> {
    /// Returns a slice of the remaining ranges in the iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let set: RangeSet<10> = [1..3, 5..7].into_iter().collect();
    /// let mut iter = set.into_iter();
    /// assert_eq!(iter.as_slice(), &[1..3, 5..7]);
    ///
    /// iter.next();
    /// assert_eq!(iter.as_slice(), &[5..7]);
    /// ```
    #[must_use]
    pub fn as_slice(&self) -> &[Range<usize>] {
        self.iter.as_slice()
    }

    /// Returns a mutable slice of the remaining ranges in the iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use range_set::RangeSet;
    ///
    /// let set: RangeSet<10> = [1..3, 5..7].into_iter().collect();
    /// let mut iter = set.into_iter();
    ///
    /// // Modify the remaining ranges
    /// for range in iter.as_mut_slice() {
    ///     range.start += 1;
    ///     range.end += 1;
    /// }
    ///
    /// let ranges: Vec<_> = iter.collect();
    /// assert_eq!(ranges, vec![2..4, 6..8]);
    /// ```
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [Range<usize>] {
        self.iter.as_mut_slice()
    }
}

impl<const CAP: usize> Iterator for IntoIter<CAP> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<const CAP: usize> DoubleEndedIterator for IntoIter<CAP> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<const CAP: usize> ExactSizeIterator for IntoIter<CAP> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, const CAP: usize> IntoIterator for &'a RangeSet<CAP> {
    type Item = &'a Range<usize>;
    type IntoIter = slice::Iter<'a, Range<usize>>;

    fn into_iter(self) -> Self::IntoIter {
        self.ranges.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn test_insert_overlapping() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..5);
        set.insert(3..7);
        assert_eq!(set.as_slice(), &[1..7]);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn test_insert_adjacent() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..3);
        set.insert(3..5);
        assert_eq!(set.as_slice(), &[1..5]);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn test_insert_multiple_merges() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..3);
        set.insert(5..7);
        set.insert(2..6);
        assert_eq!(set.as_slice(), &[1..7]);
    }

    #[test]
    fn test_insert_disjoint() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..2);
        set.insert(4..5);
        set.insert(7..8);
        assert_eq!(set.as_slice(), &[1..2, 4..5, 7..8]);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn test_insert_empty_range() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..3);
        set.insert(3..3);
        assert_eq!(set.as_slice(), &[1..3]);
    }

    #[test]
    #[should_panic(expected = "Invalid range: 5..2")]
    fn test_insert_invalid_range() {
        let mut set = RangeSet::<128>::new();
        #[expect(clippy::reversed_empty_ranges)]
        set.insert(5..2);
    }

    #[test]
    fn test_insert_at_middle() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..3);
        set.insert(7..10);
        set.insert(14..20);
        set.insert(4..6);
        assert_eq!(set.as_slice(), &[1..3, 4..6, 7..10, 14..20]);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn test_remove_non_overlapping() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..5);
        set.remove(6..8);
        assert_eq!(set.as_slice(), &[1..5]);
    }

    #[test]
    fn test_remove_exact_match() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..5);
        set.remove(1..5);
        assert_eq!(set.as_slice(), &[]);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn test_remove_partial_overlap_start() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..5);
        set.remove(1..3);
        assert_eq!(set.as_slice(), &[3..5]);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn test_remove_partial_overlap_end() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..5);
        set.remove(3..5);
        assert_eq!(set.as_slice(), &[1..3]);
    }

    #[test]
    fn test_remove_middle_split() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..10);
        set.remove(3..7);
        assert_eq!(set.as_slice(), &[1..3, 7..10]);
    }

    #[test]
    fn test_remove_multiple_ranges() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..3);
        set.insert(5..7);
        set.insert(9..11);
        set.remove(5..10);
        assert_eq!(set.as_slice(), &[1..3, 10..11]);
    }

    #[test]
    #[expect(clippy::single_range_in_vec_init)]
    fn test_remove_empty_range() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..5);
        set.remove(3..3);
        assert_eq!(set.as_slice(), &[1..5]);
    }

    #[test]
    fn test_remove_at_middle() {
        let mut set = RangeSet::<128>::new();
        set.insert(1..3);
        set.insert(4..6);
        set.insert(7..10);
        set.remove(4..6);
        assert_eq!(set.as_slice(), &[1..3, 7..10]);
    }

    #[test]
    #[should_panic(expected = "Invalid range: 5..2")]
    fn test_remove_invalid_range() {
        let mut set = RangeSet::<128>::new();
        #[expect(clippy::reversed_empty_ranges)]
        set.remove(5..2);
    }

    #[test]
    fn test_extend() {
        let mut set = RangeSet::<128>::new();
        set.extend([1..3, 4..6]);
        assert_eq!(set.as_slice(), &[1..3, 4..6]);
    }

    #[test]
    fn test_from_iter() {
        let set: RangeSet<128> = [1..3, 4..6].into_iter().collect();
        assert_eq!(set.as_slice(), &[1..3, 4..6]);
    }

    #[test]
    fn test_into_iter() {
        let set: RangeSet<128> = [1..3, 4..6].into_iter().collect();
        let ranges: Vec<_> = set.into_iter().collect();
        assert_eq!(ranges, vec![1..3, 4..6]);
    }

    #[test]
    fn test_iter() {
        let set: RangeSet<128> = [1..3, 4..6].into_iter().collect();
        let ranges: Vec<_> = set.iter().cloned().collect();
        assert_eq!(ranges, vec![1..3, 4..6]);
    }
}
