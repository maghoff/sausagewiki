use std::fmt::Debug;

use diff;
use diff::Result::*;

#[derive(Debug, PartialEq)]
enum Chunk<'a, Item: 'a + Debug + PartialEq + Eq>{
    Stable(&'a [diff::Result<Item>], &'a [diff::Result<Item>]),
    Unstable(&'a [diff::Result<Item>], &'a [diff::Result<Item>]),
}

struct MergeIterator<'a, Item>
where
    Item: 'a + Debug + PartialEq + Eq
{
    left: &'a [diff::Result<Item>],
    right: &'a [diff::Result<Item>],
}

impl<'a, Item> MergeIterator<'a, Item>
where
    Item: 'a + Debug + PartialEq + Eq
{
    fn new(left: &'a [diff::Result<Item>], right: &'a [diff::Result<Item>]) -> MergeIterator<'a, Item> {
        MergeIterator { left, right }
    }
}

impl<'a, Item> Iterator for MergeIterator<'a, Item>
where
    Item: 'a + Debug + PartialEq + Eq
{
    type Item = Chunk<'a, Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut i = 0;
        while let (Some(&Both(..)), Some(&Both(..))) = (self.left.get(i), self.right.get(i)) {
            i += 1;
        }
        if i > 0 {
            let chunk = Chunk::Stable(&self.left[..i], &self.right[..i]);
            self.left = &self.left[i..];
            self.right = &self.right[i..];
            return Some(chunk);
        }

        let mut li = 0;
        let mut ri = 0;
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
                    let chunk = Chunk::Unstable(&self.left[..li], &self.right[..ri]);
                    self.left = &self.left[li..];
                    self.right = &self.right[ri..];
                    return Some(chunk);
                }
                _ => {
                    if self.left.len() > 0 || self.right.len() > 0 {
                        let chunk = Chunk::Unstable(self.left, self.right);
                        self.left = &self.left[self.left.len()..];
                        self.right = &self.right[self.right.len()..];
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
            Chunk::Stable  (&oa[0.. 3], &ob[0.. 3]),
            Chunk::Unstable(&oa[3.. 6], &ob[3.. 3]),
            Chunk::Stable  (&oa[6.. 9], &ob[3.. 6]),
            Chunk::Unstable(&oa[9.. 9], &ob[6.. 9]),
            Chunk::Stable  (&oa[9..12], &ob[9..12]),
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
            Chunk::Stable  (&oa[0.. 3], &ob[0.. 3]),
            Chunk::Unstable(&oa[3.. 9], &ob[3.. 9]),
            Chunk::Stable  (&oa[9..12], &ob[9..12]),
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
            Chunk::Stable  (&oa[0..9], &ob[0.. 9]),
            Chunk::Unstable(&oa[9..9], &ob[9..12]),
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
            Chunk::Stable  (&oa[0..6], &ob[0.. 6]),
            Chunk::Unstable(&oa[6..9], &ob[6..12]),
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
            Chunk::Unstable(&oa[0..6], &ob[0..6]),
        ], merge);
    }
}
