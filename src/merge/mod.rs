mod chunk_iterator;
mod chunk;
mod output;

use std::fmt::Debug;

use diff;

use self::chunk_iterator::ChunkIterator;
use self::output::*;
use self::output::Output::Resolved;

pub use self::output::Output;

#[derive(Debug, PartialEq)]
pub enum MergeResult<Item: Debug + PartialEq> {
    Clean(String),
    Conflicted(Vec<Output<Item>>),
}

impl<'a> MergeResult<&'a str> {
    pub fn to_strings(self) -> MergeResult<String> {
        match self {
            MergeResult::Clean(x) => MergeResult::Clean(x),
            MergeResult::Conflicted(x) => MergeResult::Conflicted(
                x.into_iter().map(Output::to_strings).collect()
            )
        }
    }
}

pub fn merge_lines<'a>(a: &'a str, o: &'a str, b: &'a str) -> MergeResult<&'a str> {
    let oa = diff::lines(o, a);
    let ob = diff::lines(o, b);

    let chunks = ChunkIterator::new(&oa, &ob);
    let hunks: Vec<_> = chunks.map(resolve).collect();

    let clean = hunks.iter().all(|x| match x { &Resolved(..) => true, _ => false });

    if clean {
        MergeResult::Clean(
            hunks
                .into_iter()
                .flat_map(|x| match x {
                    Resolved(y) => y.into_iter(),
                    _ => unreachable!()
                })
                .flat_map(|x| vec![x, "\n"].into_iter())
                .collect()
        )
    } else {
        MergeResult::Conflicted(hunks)
    }
}

pub fn merge_chars<'a>(a: &'a str, o: &'a str, b: &'a str) -> MergeResult<char> {
    let oa = diff::chars(o, a);
    let ob = diff::chars(o, b);

    let chunks = ChunkIterator::new(&oa, &ob);
    let hunks: Vec<_> = chunks.map(resolve).collect();

    let clean = hunks.iter().all(|x| match x { &Resolved(..) => true, _ => false });

    if clean {
        MergeResult::Clean(
            hunks
                .into_iter()
                .flat_map(|x| match x {
                    Resolved(y) => y.into_iter(),
                    _ => unreachable!()
                })
                .collect()
        )
    } else {
        MergeResult::Conflicted(hunks)
    }
}

#[cfg(test)]
mod test {
    use diff;

    use super::*;
    use super::output::*;
    use super::output::Output::*;

    #[test]
    fn simple_case() {
        fn merge_chars(a: &str, o: &str, b: &str) -> Vec<Output<char>> {
            let oa = diff::chars(o, a);
            let ob = diff::chars(o, b);

            let chunks = super::chunk_iterator::ChunkIterator::new(&oa, &ob);
            chunks.map(resolve).collect()
        }

        assert_eq!(vec![
            Resolved("aaa".chars().collect()),
            Resolved("xxx".chars().collect()),
            Resolved("bbb".chars().collect()),
            Resolved("yyy".chars().collect()),
            Resolved("ccc".chars().collect()),
        ], merge_chars(
            "aaaxxxbbbccc",
            "aaabbbccc",
            "aaabbbyyyccc",
        ));
    }

    #[test]
    fn clean_case() {
        assert_eq!(MergeResult::Clean("\
            aaa\n\
            xxx\n\
            bbb\n\
            yyy\n\
            ccc\n\
        ".into()), merge_lines(
            "aaa\nxxx\nbbb\nccc\n",
            "aaa\nbbb\nccc\n",
            "aaa\nbbb\nyyy\nccc\n",
        ));
    }

    #[test]
    fn clean_case_chars() {
        assert_eq!(MergeResult::Clean("Title".into()), merge_chars(
            "Titlle",
            "titlle",
            "title",
        ));
    }

    #[test]
    fn false_conflict() {
        assert_eq!(MergeResult::Clean("\
            aaa\n\
            xxx\n\
            ccc\n\
        ".into()), merge_lines(
            "aaa\nxxx\nccc\n",
            "aaa\nbbb\nccc\n",
            "aaa\nxxx\nccc\n",
        ));
    }

    #[test]
    fn true_conflict() {
        assert_eq!(MergeResult::Conflicted(vec![
            Resolved(vec!["aaa"]),
            Conflict(vec!["xxx"], vec![], vec!["yyy"]),
            Resolved(vec!["bbb", "ccc"]),
        ]), merge_lines(
            "aaa\nxxx\nbbb\nccc\n",
            "aaa\nbbb\nccc\n",
            "aaa\nyyy\nbbb\nccc\n",
        ));
    }
}
