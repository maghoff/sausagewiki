use std::fmt::Debug;

use diff;
use diff::Result::*;

#[derive(Debug, PartialEq)]
struct Chunk<'a, Item: 'a + Debug + PartialEq + Eq>(&'a [diff::Result<Item>], &'a [diff::Result<Item>]);

#[derive(Debug, PartialEq)]
enum ChunkKind {
    Stable,
    Unstable,
}

#[derive(Debug, PartialEq)]
struct ChunkItem<'a, Item>
where
    Item: 'a + Debug + PartialEq + Eq
{
    kind: ChunkKind,
    chunk: Chunk<'a, Item>,
}

impl<'a, Item> ChunkItem<'a, Item>
where
    Item: 'a + Debug + PartialEq + Eq
{
    fn stable(chunk: Chunk<'a, Item>) -> ChunkItem<'a, Item> {
        ChunkItem {
            kind: ChunkKind::Stable,
            chunk: chunk,
        }
    }

    fn unstable(chunk: Chunk<'a, Item>) -> ChunkItem<'a, Item> {
        ChunkItem {
            kind: ChunkKind::Unstable,
            chunk: chunk,
        }
    }
}

struct MergeIterator<'a, Item>
where
    Item: 'a + Debug + PartialEq + Eq
{
    left: &'a [diff::Result<Item>],
    right: &'a [diff::Result<Item>],

    li: usize,
    ri: usize,
}

impl<'a, Item> MergeIterator<'a, Item>
where
    Item: 'a + Debug + PartialEq + Eq
{
    fn new(left: &'a [diff::Result<Item>], right: &'a [diff::Result<Item>]) -> MergeIterator<'a, Item> {
        MergeIterator {
            left,
            right,
            li: 0,
            ri: 0,
        }
    }
}

impl<'a, Item> Iterator for MergeIterator<'a, Item>
where
    Item: 'a + Debug + PartialEq + Eq
{
    type Item = ChunkItem<'a, Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut i = 0;
        while let (Some(&Both(..)), Some(&Both(..))) = (self.left.get(self.li+i), self.right.get(self.ri+i)) {
            i += 1;
        }
        if i > 0 {
            let chunk = ChunkItem::stable(Chunk(&self.left[self.li..self.li+i], &self.right[self.ri..self.ri+i]));
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
                    let chunk = ChunkItem::unstable(Chunk(&self.left[self.li..li], &self.right[self.ri..ri]));
                    self.li = li;
                    self.ri = ri;
                    return Some(chunk);
                }
                _ => {
                    if self.li < self.left.len() || self.ri < self.right.len() {
                        let chunk = ChunkItem::unstable(Chunk(&self.left[self.li..self.left.len()], &self.right[self.ri..self.right.len()]));
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

        let merge = MergeIterator::new(&oa, &ob).collect::<Vec<_>>();

        assert_eq!(vec![
            ChunkItem::stable  (Chunk(&oa[0.. 3], &ob[0.. 3])),
            ChunkItem::unstable(Chunk(&oa[3.. 6], &ob[3.. 3])),
            ChunkItem::stable  (Chunk(&oa[6.. 9], &ob[3.. 6])),
            ChunkItem::unstable(Chunk(&oa[9.. 9], &ob[6.. 9])),
            ChunkItem::stable  (Chunk(&oa[9..12], &ob[9..12])),
        ], merge);
    }

    #[test]
    fn real_conflict() {
        let o = "aaabbbccc";
        let a = "aaaxxxccc";
        let b = "aaayyyccc";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(&oa, &ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::stable  (Chunk(&oa[0.. 3], &ob[0.. 3])),
            ChunkItem::unstable(Chunk(&oa[3.. 9], &ob[3.. 9])),
            ChunkItem::stable  (Chunk(&oa[9..12], &ob[9..12])),
        ], merge);
    }

    #[test]
    fn additional_at_end() {
        let o = "aaabbbccc";
        let a = "aaabbbccc";
        let b = "aaabbbcccddd";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(&oa, &ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::stable  (Chunk(&oa[0..9], &ob[0.. 9])),
            ChunkItem::unstable(Chunk(&oa[9..9], &ob[9..12])),
        ], merge);
    }

    #[test]
    fn additional_at_end_2() {
        let o = "aaabbb";
        let a = "aaabbbccc";
        let b = "aaabbbcccddd";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(&oa, &ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::stable  (Chunk(&oa[0..6], &ob[0.. 6])),
            ChunkItem::unstable(Chunk(&oa[6..9], &ob[6..12])),
        ], merge);
    }

    #[test]
    fn completely_unrelated() {
        let o = "aaa";
        let a = "bbb";
        let b = "ccc";

        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = MergeIterator::new(&oa, &ob).collect::<Vec<_>>();
        assert_eq!(vec![
            ChunkItem::unstable(Chunk(&oa[0..6], &ob[0..6])),
        ], merge);
    }
}
