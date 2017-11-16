use std::ops::Range;

use diff;
use diff::Result::*;

#[derive(Debug, PartialEq, Eq)]
struct Chunk(Range<usize>, Range<usize>);

#[derive(Debug, PartialEq, Eq)]
enum ChunkKind {
    Stable,
    Unstable,
}

#[derive(Debug, PartialEq, Eq)]
struct ChunkItem {
    kind: ChunkKind,
    chunk: Chunk,
}

impl ChunkItem {
    fn stable(chunk: Chunk) -> ChunkItem {
        ChunkItem {
            kind: ChunkKind::Stable,
            chunk: chunk,
        }
    }

    fn unstable(chunk: Chunk) -> ChunkItem {
        ChunkItem {
            kind: ChunkKind::Unstable,
            chunk: chunk,
        }
    }
}

struct MergeIterator<Item> {
    left: Vec<diff::Result<Item>>,
    right: Vec<diff::Result<Item>>,

    li: usize,
    ri: usize,
}

impl<Item> MergeIterator<Item> {
    fn new(left: Vec<diff::Result<Item>>, right: Vec<diff::Result<Item>>) -> MergeIterator<Item> {
        MergeIterator {
            left,
            right,
            li: 0,
            ri: 0,
        }
    }
}

impl<Item> Iterator for MergeIterator<Item>
where
    Item: ::std::fmt::Debug + PartialEq
{
    type Item = ChunkItem;

    fn next(&mut self) -> Option<Self::Item> {
        let mut i = 0;
        while let (Some(&Both(..)), Some(&Both(..))) = (self.left.get(self.li+i), self.right.get(self.ri+i)) {
            i += 1;
        }
        if i > 0 {
            let chunk = ChunkItem::stable(Chunk(self.li..self.li+i, self.ri..self.ri+i));
            self.li += i;
            self.ri += i;
            return Some(chunk);
        }

        let mut li = self.li;
        let mut ri = self.ri;
        loop {
            match (self.left.get(li), self.right.get(ri)) {
                (Some(&Right(_)), _) => {
                    li += 1;
                },
                (_, Some(&Right(_))) => {
                    ri += 1;
                },
                (Some(&Left(_)), Some(_)) => {
                    li += 1;
                    ri += 1;
                },
                (Some(_), Some(&Left(_))) => {
                    li += 1;
                    ri += 1;
                },
                (Some(&Both(..)), Some(&Both(..))) => {
                    let chunk = ChunkItem::unstable(Chunk(self.li..li, self.ri..ri));
                    self.li = li;
                    self.ri = ri;
                    return Some(chunk);
                }
                _ => {
                    if self.li < self.left.len() || self.ri < self.right.len() {
                        let chunk = ChunkItem::unstable(Chunk(self.li..self.left.len(), self.ri..self.right.len()));
                        self.li = self.left.len();
                        self.ri = self.right.len();
                        return Some(chunk);
                    }
                    return None;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use diff;

    #[test]
    fn simple_case() {
        let o = "aaabbbccc";
        let a = "aaaxxxbbbccc";
        let b = "aaabbbyyyccc";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(oa, ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::stable(Chunk(0..3, 0..3)),
            ChunkItem::unstable(Chunk(3..6, 3..3)),
            ChunkItem::stable(Chunk(6..9, 3..6)),
            ChunkItem::unstable(Chunk(9..9, 6..9)),
            ChunkItem::stable(Chunk(9..12, 9..12)),
        ], merge);
    }

    #[test]
    fn real_conflict() {
        let o = "aaabbbccc";
        let a = "aaaxxxccc";
        let b = "aaayyyccc";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(oa, ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::stable(Chunk(0..3, 0..3)),
            ChunkItem::unstable(Chunk(3..9, 3..9)),
            ChunkItem::stable(Chunk(9..12, 9..12)),
        ], merge);
    }

    #[test]
    fn additional_at_end() {
        let o = "aaabbbccc";
        let a = "aaabbbccc";
        let b = "aaabbbcccddd";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(oa, ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::stable(Chunk(0..9, 0..9)),
            ChunkItem::unstable(Chunk(9..9, 9..12)),
        ], merge);
    }

    #[test]
    fn additional_at_end_2() {
        let o = "aaabbb";
        let a = "aaabbbccc";
        let b = "aaabbbcccddd";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(oa, ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::stable(Chunk(0..6, 0..6)),
            ChunkItem::unstable(Chunk(6..9, 6..12)),
        ], merge);
    }

    #[test]
    fn completely_unrelated() {
        let o = "aaa";
        let a = "bbb";
        let b = "ccc";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(oa, ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::unstable(Chunk(0..6, 0..6)),
        ], merge);
    }
}
