mod parse;
use parse::ParseRangesError;
use std::cmp::Ordering;
use std::convert::From;
use std::str::FromStr;
use std::vec::Vec;

/// A range of bytes, characters, or fields to select from the input.
pub(crate) enum CutRange {
    /// A single element.
    Unit(usize),
    /// A closed range of elements.
    Closed(IncreasingRange),
    /// All elements from the start to the specified end.
    FromStart(usize),
    /// All elements from the specified start to the end.
    ToEnd(usize),
}

/// An increasing range.
pub(crate) struct IncreasingRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl IncreasingRange {
    /// Creates a new `IncreasingRange` with the specified start and end.
    fn new(start: usize, end: usize) -> IncreasingRange {
        assert!(start <= end);
        IncreasingRange { start, end }
    }
}

/// A set of simplified and merged `CutRange`s. See [`MergedRange`].
#[derive(Debug, Clone)]
pub(crate) struct Ranges {
    pub(crate) ranges: Vec<MergedRange>,
}

impl PartialEq for Ranges {
    fn eq(&self, other: &Ranges) -> bool {
        self.ranges == other.ranges
    }
}

impl Eq for Ranges {}

/// Simplified view of one or more merged `CutRange`s.
#[derive(Copy, Clone, Debug)]
pub(crate) enum MergedRange {
    Closed(usize, usize),
    ToEnd(usize),
}

impl From<&CutRange> for MergedRange {
    fn from(range: &CutRange) -> Self {
        match *range {
            CutRange::FromStart(end) => MergedRange::Closed(0usize, end),
            CutRange::Unit(n) => MergedRange::Closed(n, n),
            CutRange::ToEnd(start) => MergedRange::ToEnd(start),
            CutRange::Closed(IncreasingRange { start, end }) => MergedRange::Closed(start, end),
        }
    }
}

impl PartialEq for MergedRange {
    fn eq(&self, other: &MergedRange) -> bool {
        use MergedRange::{Closed, ToEnd};
        match (self, other) {
            (ToEnd(s1), ToEnd(s2)) => s1 == s2,
            (Closed(s1, e1), Closed(s2, e2)) => s1 == s2 && e1 == e2,
            _ => false,
        }
    }
}

impl Eq for MergedRange {}

impl Ord for MergedRange {
    fn cmp(&self, other: &Self) -> Ordering {
        use MergedRange::{Closed, ToEnd};
        match (self, other) {
            (ToEnd(s1), ToEnd(s2)) => s1.cmp(s2),
            (Closed(s1, e1), Closed(s2, e2)) => s1.cmp(s2).then(e1.cmp(e2)),
            (ToEnd(s1), Closed(s2, _)) => s1.cmp(s2).then(Ordering::Greater),
            (Closed(s1, _), ToEnd(s2)) => s1.cmp(s2).then(Ordering::Less),
        }
    }
}

impl PartialOrd for MergedRange {
    fn partial_cmp(&self, other: &MergedRange) -> Option<Ordering> {
        Option::Some(self.cmp(other))
    }
}

/// A collection of ranges.
impl Ranges {
    /// Create a new `Ranges` from `CutRange`s.
    pub(crate) fn from_ranges(ranges: &[CutRange]) -> Ranges {
        if ranges.is_empty() {
            return Ranges { ranges: Vec::new() };
        }

        // Simplify ranges and sort for easier merging.
        let mut sorted_ranges: Vec<MergedRange> = ranges.iter().map(|cr| cr.into()).collect();
        sorted_ranges.sort();

        let mut result = Vec::new();

        let mut ranges_iter = sorted_ranges.iter();
        // Current range which may be merged with next range.
        let mut merge_chain = *ranges_iter.next().unwrap_or(&MergedRange::ToEnd(0usize));

        // Merge
        for next in ranges_iter {
            use MergedRange::{Closed, ToEnd};
            match (merge_chain, next) {
                // Ordering of MergedRanges ensures that all later ranges will start after the start
                // of the current ToEnd, so all remaining ranges can be merged.
                // e.g. 4-,4-8,6-,9-12 is reduced to 4-.
                (ToEnd(_), _) => break,
                (Closed(s1, e1), ToEnd(s2)) => {
                    // assert s1 <= s2

                    // Current Closed overlaps or touches next ToEnd, which we merge into ToEnd(s1).
                    // Sort order ensure that all following values can also be merged (see comment
                    // for previous match arm).
                    if *s2 <= e1 + 1 {
                        merge_chain = ToEnd(s1);
                        break;
                    }
                    // No overlap. Add current range then stat a new merge chain.
                    else {
                        result.push(merge_chain);
                        merge_chain = *next;
                        break;
                    }
                }
                (Closed(s1, e1), Closed(s2, e2)) => {
                    // assert s1 <= s2

                    // Ranges overlap or touch. Merge and continue.
                    if *s2 <= e1 + 1 {
                        merge_chain = Closed(s1, std::cmp::max(e1, *e2));
                    }
                    // No overlap. Add current range then start a new merge chain.
                    else {
                        result.push(merge_chain);
                        merge_chain = *next;
                    }
                }
            }
        }

        // Add final merge chain.
        result.push(merge_chain);

        Ranges { ranges: result }
    }

    pub(crate) fn complement(self) -> Ranges {
        let mut next = 0usize;
        let mut open = false;
        let mut ranges = Vec::new();

        for range in self.ranges {
            match range {
                MergedRange::Closed(start, end) => {
                    if start != 0 {
                        ranges.push(MergedRange::Closed(next, start - 1));
                    }
                    next = end + 1;
                }
                MergedRange::ToEnd(start) => {
                    if start != 0 {
                        ranges.push(MergedRange::Closed(next, start - 1));
                    }
                    open = true;
                }
            }
        }

        if !open {
            ranges.push(MergedRange::ToEnd(next));
        }
        Ranges { ranges }
    }
}

impl IntoIterator for Ranges {
    type Item = MergedRange;
    type IntoIter = std::vec::IntoIter<MergedRange>;
    fn into_iter(self) -> Self::IntoIter {
        self.ranges.into_iter()
    }
}

impl FromStr for Ranges {
    type Err = ParseRangesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_no_ranges() {
        let ranges = Ranges::from_ranges(&[]);
        assert_eq!(ranges.into_iter().next(), Option::None);
    }

    #[test]
    fn from_single_range() {
        use CutRange::{FromStart, ToEnd, Unit};

        assert_simplify_to_single_range(&[Unit(0)], MergedRange::Closed(0, 0));
        assert_simplify_to_single_range(&[Unit(3)], MergedRange::Closed(3, 3));
        assert_simplify_to_single_range(&[Unit(9)], MergedRange::Closed(9, 9));
        assert_simplify_to_single_range(&[Unit(15)], MergedRange::Closed(15, 15));

        assert_simplify_to_single_range(&[closed(0, 0)], MergedRange::Closed(0, 0));
        assert_simplify_to_single_range(&[closed(0, 1)], MergedRange::Closed(0, 1));
        assert_simplify_to_single_range(&[closed(1, 4)], MergedRange::Closed(1, 4));
        assert_simplify_to_single_range(&[closed(8, 32)], MergedRange::Closed(8, 32));

        assert_simplify_to_single_range(&[FromStart(0)], MergedRange::Closed(0, 0));
        assert_simplify_to_single_range(&[FromStart(1)], MergedRange::Closed(0, 1));
        assert_simplify_to_single_range(&[FromStart(4)], MergedRange::Closed(0, 4));
        assert_simplify_to_single_range(&[FromStart(8)], MergedRange::Closed(0, 8));

        assert_simplify_to_single_range(&[ToEnd(0)], MergedRange::ToEnd(0));
        assert_simplify_to_single_range(&[ToEnd(1)], MergedRange::ToEnd(1));
        assert_simplify_to_single_range(&[ToEnd(4)], MergedRange::ToEnd(4));
        assert_simplify_to_single_range(&[ToEnd(8)], MergedRange::ToEnd(8));
    }

    #[test]
    fn simplify_sorted_to_single_range() {
        use CutRange::{FromStart, ToEnd, Unit};

        // Units only.
        assert_simplify_to_single_range(&[Unit(0), Unit(1)], MergedRange::Closed(0, 1));
        assert_simplify_to_single_range(&[Unit(5), Unit(6)], MergedRange::Closed(5, 6));
        assert_simplify_to_single_range(
            &[Unit(9), Unit(10), Unit(11), Unit(12)],
            MergedRange::Closed(9, 12),
        );

        // Closed only.
        assert_simplify_to_single_range(&[closed(0, 2), closed(3, 5)], MergedRange::Closed(0, 5));
        assert_simplify_to_single_range(&[closed(0, 4), closed(4, 7)], MergedRange::Closed(0, 7));
        assert_simplify_to_single_range(
            &[closed(3, 6), closed(5, 8), closed(9, 10)],
            MergedRange::Closed(3, 10),
        );

        // FromStart only.
        assert_simplify_to_single_range(&[FromStart(2), FromStart(5)], MergedRange::Closed(0, 5));
        assert_simplify_to_single_range(&[FromStart(6), FromStart(9)], MergedRange::Closed(0, 9));

        // ToEnd only.
        assert_simplify_to_single_range(&[ToEnd(4), ToEnd(6)], MergedRange::ToEnd(4));
        assert_simplify_to_single_range(&[ToEnd(5), ToEnd(9)], MergedRange::ToEnd(5));

        // Unit and Closed.
        assert_simplify_to_single_range(&[Unit(0), closed(1, 4)], MergedRange::Closed(0, 4));
        assert_simplify_to_single_range(&[closed(6, 9), Unit(10)], MergedRange::Closed(6, 10));
        assert_simplify_to_single_range(
            &[closed(12, 15), Unit(16), closed(17, 19)],
            MergedRange::Closed(12, 19),
        );

        // FromStart and Unit.
        assert_simplify_to_single_range(&[FromStart(3), Unit(3)], MergedRange::Closed(0, 3));
        assert_simplify_to_single_range(&[FromStart(4), Unit(5)], MergedRange::Closed(0, 5));
        assert_simplify_to_single_range(
            &[FromStart(5), Unit(6), Unit(7)],
            MergedRange::Closed(0, 7),
        );

        // FromStart and Closed.
        assert_simplify_to_single_range(&[FromStart(3), closed(4, 5)], MergedRange::Closed(0, 5));
        assert_simplify_to_single_range(&[FromStart(6), closed(3, 10)], MergedRange::Closed(0, 10));

        // Unit and ToEnd.
        assert_simplify_to_single_range(&[Unit(4), CutRange::ToEnd(4)], MergedRange::ToEnd(4));
        assert_simplify_to_single_range(&[Unit(5), CutRange::ToEnd(6)], MergedRange::ToEnd(5));
        assert_simplify_to_single_range(
            &[CutRange::ToEnd(6), Unit(7), Unit(10)],
            MergedRange::ToEnd(6),
        );

        // Closed and ToEnd.
        assert_simplify_to_single_range(&[closed(3, 5), CutRange::ToEnd(5)], MergedRange::ToEnd(3));
        assert_simplify_to_single_range(&[closed(5, 7), CutRange::ToEnd(6)], MergedRange::ToEnd(5));
        assert_simplify_to_single_range(
            &[CutRange::ToEnd(7), closed(8, 12)],
            MergedRange::ToEnd(7),
        );

        // FromStart and ToEnd.
        assert_simplify_to_single_range(&[FromStart(3), ToEnd(3)], MergedRange::ToEnd(0));
        assert_simplify_to_single_range(&[FromStart(5), ToEnd(6)], MergedRange::ToEnd(0));
        assert_simplify_to_single_range(&[FromStart(7), ToEnd(3)], MergedRange::ToEnd(0));

        // All
        assert_simplify_to_single_range(
            &[Unit(0), FromStart(3), closed(4, 7), ToEnd(8)],
            MergedRange::ToEnd(0),
        );
        assert_simplify_to_single_range(
            &[FromStart(5), Unit(2), closed(4, 7), ToEnd(3)],
            MergedRange::ToEnd(0),
        );
    }

    #[test]
    fn simplify_unsorted_to_single_range() {
        use CutRange::{FromStart, ToEnd, Unit};

        // Single type.
        assert_simplify_to_single_range(
            &[Unit(2), Unit(0), Unit(1), Unit(3)],
            MergedRange::Closed(0, 3),
        );
        assert_simplify_to_single_range(
            &[closed(3, 5), closed(1, 3), closed(6, 9)],
            MergedRange::Closed(1, 9),
        );
        assert_simplify_to_single_range(
            &[closed(7, 11), closed(5, 12), closed(9, 14)],
            MergedRange::Closed(5, 14),
        );
        assert_simplify_to_single_range(&[FromStart(10), FromStart(5)], MergedRange::Closed(0, 10));
        assert_simplify_to_single_range(&[ToEnd(6), ToEnd(4)], MergedRange::ToEnd(4));

        // Mixed types.
        assert_simplify_to_single_range(
            &[Unit(6), closed(3, 8), Unit(9)],
            MergedRange::Closed(3, 9),
        );
        assert_simplify_to_single_range(
            &[Unit(5), FromStart(3), closed(4, 4)],
            MergedRange::Closed(0, 5),
        );
        assert_simplify_to_single_range(&[ToEnd(10), closed(5, 8), Unit(9)], MergedRange::ToEnd(5));
    }

    #[test]
    fn simplify_sorted_to_multiple_ranges() {
        use CutRange::{FromStart, ToEnd, Unit};

        // Single type.
        assert_simplify_to_multiple_ranges(
            &[Unit(1), Unit(2), Unit(6), Unit(7), Unit(15)],
            &[
                MergedRange::Closed(1, 2),
                MergedRange::Closed(6, 7),
                MergedRange::Closed(15, 15),
            ],
        );
        assert_simplify_to_multiple_ranges(
            &[closed(3, 5), closed(6, 7), closed(10, 12), closed(11, 14)],
            &[MergedRange::Closed(3, 7), MergedRange::Closed(10, 14)],
        );

        // Mixed types.
        assert_simplify_to_multiple_ranges(
            &[closed(2, 6), Unit(7), Unit(10), closed(10, 15)],
            &[MergedRange::Closed(2, 7), MergedRange::Closed(10, 15)],
        );
        assert_simplify_to_multiple_ranges(
            &[FromStart(3), Unit(4), closed(7, 12), Unit(13)],
            &[MergedRange::Closed(0, 4), MergedRange::Closed(7, 13)],
        );
        assert_simplify_to_multiple_ranges(
            &[FromStart(3), ToEnd(6)],
            &[MergedRange::Closed(0, 3), MergedRange::ToEnd(6)],
        );
        assert_simplify_to_multiple_ranges(
            &[FromStart(2), Unit(4), ToEnd(8)],
            &[
                MergedRange::Closed(0, 2),
                MergedRange::Closed(4, 4),
                MergedRange::ToEnd(8),
            ],
        );
        assert_simplify_to_multiple_ranges(
            &[FromStart(5), closed(10, 15), ToEnd(20)],
            &[
                MergedRange::Closed(0, 5),
                MergedRange::Closed(10, 15),
                MergedRange::ToEnd(20),
            ],
        );
        assert_simplify_to_multiple_ranges(
            &[closed(2, 5), Unit(6), Unit(9), ToEnd(10)],
            &[MergedRange::Closed(2, 6), MergedRange::ToEnd(9)],
        );
        assert_simplify_to_multiple_ranges(
            &[
                FromStart(1),
                closed(4, 6),
                Unit(9),
                closed(10, 12),
                ToEnd(16),
            ],
            &[
                MergedRange::Closed(0, 1),
                MergedRange::Closed(4, 6),
                MergedRange::Closed(9, 12),
                MergedRange::ToEnd(16),
            ],
        );
    }

    #[test]
    fn simplify_unsorted_to_multiple_ranges() {
        use CutRange::{FromStart, ToEnd, Unit};

        // Single Type.
        assert_simplify_to_multiple_ranges(
            &[Unit(5), Unit(3), Unit(11), Unit(10), Unit(4)],
            &[MergedRange::Closed(3, 5), MergedRange::Closed(10, 11)],
        );
        assert_simplify_to_multiple_ranges(
            &[closed(1, 4), closed(9, 11), closed(8, 10), closed(5, 6)],
            &[MergedRange::Closed(1, 6), MergedRange::Closed(8, 11)],
        );

        // Multiple types.
        assert_simplify_to_multiple_ranges(
            &[closed(5, 6), closed(9, 12), Unit(3), closed(3, 4), Unit(10)],
            &[MergedRange::Closed(3, 6), MergedRange::Closed(9, 12)],
        );
        assert_simplify_to_multiple_ranges(
            &[closed(9, 12), Unit(6), Unit(13), FromStart(5)],
            &[MergedRange::Closed(0, 6), MergedRange::Closed(9, 13)],
        );
        assert_simplify_to_multiple_ranges(
            &[ToEnd(9), FromStart(4)],
            &[MergedRange::Closed(0, 4), MergedRange::ToEnd(9)],
        );
        assert_simplify_to_multiple_ranges(
            &[closed(10, 15), FromStart(5), ToEnd(20), Unit(6)],
            &[
                MergedRange::Closed(0, 6),
                MergedRange::Closed(10, 15),
                MergedRange::ToEnd(20),
            ],
        );
        assert_simplify_to_multiple_ranges(
            &[
                closed(12, 14),
                Unit(15),
                ToEnd(20),
                Unit(4),
                closed(19, 19),
                FromStart(6),
                closed(5, 8),
                Unit(18),
            ],
            &[
                MergedRange::Closed(0, 8),
                MergedRange::Closed(12, 15),
                MergedRange::ToEnd(18),
            ],
        );
    }

    #[test]
    fn complement_empty() {
        let ranges: Ranges = "1-".parse().unwrap();
        let completement = ranges.complement();
        assert!(completement.ranges.is_empty());
    }

    #[test]
    fn complement_single_range() {
        assert_complement("1", "2-");
        assert_complement("2", "1,3-");
        assert_complement("3", "1-2,4-");
        assert_complement("4", "1-3,5-");
        assert_complement("1-2", "3-");
        assert_complement("3-4", "1-2,5-");
        assert_complement("5-10", "1-4,11-");
    }

    #[test]
    fn complement_multiple_ranges() {
        assert_complement("1,3", "2,4-");
        assert_complement("2,4,6,8", "1,3,5,7,9-");
        assert_complement("1-3,5-7", "4,8-");
        assert_complement("2-4,8-16", "1,5-7,17-");
        assert_complement("1-10,20-", "11-19");
        assert_complement("3-6,10-20,40-", "1-2,7-9,21-39");
    }

    fn assert_simplify_to_single_range(input_ranges: &[CutRange], expected_range: MergedRange) {
        let actual_ranges = Ranges::from_ranges(input_ranges);
        let mut elements = actual_ranges.into_iter();
        assert_eq!(elements.next(), Option::Some(expected_range));
        assert_eq!(elements.next(), Option::None);
    }

    fn assert_simplify_to_multiple_ranges(
        input_ranges: &[CutRange],
        expected_ranges: &[MergedRange],
    ) {
        let actual_ranges = Ranges::from_ranges(input_ranges);
        let mut elements = actual_ranges.into_iter();

        for expected_range in expected_ranges {
            assert_eq!(elements.next().unwrap(), *expected_range);
        }
        assert_eq!(elements.next(), Option::None);
    }

    fn assert_complement(ranges: &str, complement: &str) {
        let actual = ranges.parse::<Ranges>().unwrap().complement();
        let expected = complement.parse().unwrap();
        assert_eq!(actual, expected);
    }

    // Helper function to simplify the creation of CutRange::Closed.
    fn closed(start: usize, end: usize) -> CutRange {
        CutRange::Closed(IncreasingRange::new(start, end))
    }
}
