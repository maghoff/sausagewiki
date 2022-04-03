mod chunk;
mod chunk_iterator;
mod output;

use std::fmt::Debug;

use diff;

use self::chunk_iterator::ChunkIterator;
use self::output::Output::Resolved;
use self::output::*;

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
            MergeResult::Conflicted(x) => {
                MergeResult::Conflicted(x.into_iter().map(Output::to_strings).collect())
            }
        }
    }
}

impl MergeResult<String> {
    pub fn flatten(self) -> String {
        match self {
            MergeResult::Clean(x) => x,
            MergeResult::Conflicted(x) => x
                .into_iter()
                .flat_map(|out| match out {
                    Output::Conflict(a, _o, b) => {
                        let mut x: Vec<String> = vec![];
                        x.push("<<<<<<< Your changes:\n".into());
                        x.extend(a.into_iter().map(|x| format!("{}\n", x)));
                        x.push("======= Their changes:\n".into());
                        x.extend(b.into_iter().map(|x| format!("{}\n", x)));
                        x.push(">>>>>>> Conflict ends here\n".into());
                        x
                    }
                    Output::Resolved(x) => x.into_iter().map(|x| format!("{}\n", x)).collect(),
                })
                .collect(),
        }
    }
}

impl MergeResult<char> {
    pub fn flatten(self) -> String {
        match self {
            MergeResult::Clean(x) => x,
            MergeResult::Conflicted(x) => x
                .into_iter()
                .flat_map(|out| match out {
                    Output::Conflict(a, _o, b) => {
                        let mut x: Vec<char> = vec![];
                        x.push('<');
                        x.extend(a);
                        x.push('|');
                        x.extend(b);
                        x.push('>');
                        x
                    }
                    Output::Resolved(x) => x,
                })
                .collect(),
        }
    }
}

pub fn merge_lines<'a>(a: &'a str, o: &'a str, b: &'a str) -> MergeResult<&'a str> {
    let oa = diff::lines(o, a);
    let ob = diff::lines(o, b);

    let chunks = ChunkIterator::new(&oa, &ob);
    let hunks: Vec<_> = chunks.map(resolve).collect();

    let clean = hunks.iter().all(|x| match x {
        &Resolved(..) => true,
        _ => false,
    });

    if clean {
        MergeResult::Clean(
            hunks
                .into_iter()
                .flat_map(|x| match x {
                    Resolved(y) => y.into_iter(),
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>()
                .join("\n"),
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

    let clean = hunks.iter().all(|x| match x {
        &Resolved(..) => true,
        _ => false,
    });

    if clean {
        MergeResult::Clean(
            hunks
                .into_iter()
                .flat_map(|x| match x {
                    Resolved(y) => y.into_iter(),
                    _ => unreachable!(),
                })
                .collect(),
        )
    } else {
        MergeResult::Conflicted(hunks)
    }
}

#[cfg(test)]
mod test {
    use diff;

    use super::output::Output::*;
    use super::output::*;
    use super::*;

    #[test]
    fn simple_case() {
        fn merge_chars(a: &str, o: &str, b: &str) -> Vec<Output<char>> {
            let oa = diff::chars(o, a);
            let ob = diff::chars(o, b);

            let chunks = super::chunk_iterator::ChunkIterator::new(&oa, &ob);
            chunks.map(resolve).collect()
        }

        assert_eq!(
            vec![
                Resolved("aaa".chars().collect()),
                Resolved("xxx".chars().collect()),
                Resolved("bbb".chars().collect()),
                Resolved("yyy".chars().collect()),
                Resolved("ccc".chars().collect()),
            ],
            merge_chars("aaaxxxbbbccc", "aaabbbccc", "aaabbbyyyccc",)
        );
    }

    #[test]
    fn clean_case() {
        assert_eq!(
            MergeResult::Clean(
                indoc!(
                    "
            aaa
            xxx
            bbb
            yyy
            ccc
        "
                )
                .into()
            ),
            merge_lines(
                indoc!(
                    "
                aaa
                xxx
                bbb
                ccc
            "
                ),
                indoc!(
                    "
                aaa
                bbb
                ccc
            "
                ),
                indoc!(
                    "
                aaa
                bbb
                yyy
                ccc
            "
                ),
            )
        );
    }

    #[test]
    fn clean_case_chars() {
        assert_eq!(
            MergeResult::Clean("Title".into()),
            merge_chars("Titlle", "titlle", "title",)
        );
    }

    #[test]
    fn false_conflict() {
        assert_eq!(
            MergeResult::Clean(
                indoc!(
                    "
            aaa
            xxx
            ccc
        "
                )
                .into()
            ),
            merge_lines(
                indoc!(
                    "
                aaa
                xxx
                ccc
            "
                ),
                indoc!(
                    "
                aaa
                bbb
                ccc
            "
                ),
                indoc!(
                    "
                aaa
                xxx
                ccc
            "
                ),
            )
        );
    }

    #[test]
    fn true_conflict() {
        assert_eq!(
            MergeResult::Conflicted(vec![
                Resolved(vec!["aaa"]),
                Conflict(vec!["xxx"], vec![], vec!["yyy"]),
                Resolved(vec!["bbb", "ccc", ""]),
            ]),
            merge_lines(
                indoc!(
                    "
                aaa
                xxx
                bbb
                ccc
            "
                ),
                indoc!(
                    "
                aaa
                bbb
                ccc
            "
                ),
                indoc!(
                    "
                aaa
                yyy
                bbb
                ccc
            "
                ),
            )
        );
    }
}
