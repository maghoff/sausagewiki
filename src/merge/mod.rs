mod chunk_iterator;
mod chunk;
mod output;

#[cfg(test)]
mod test {
    use diff;
    use super::output::*;
    use super::output::Output::*;

    fn merge_chars(a: &str, o: &str, b: &str) -> Vec<Output<char>> {
        let oa = diff::chars(o, a);
        let ob = diff::chars(o, b);

        let merge = super::chunk_iterator::ChunkIterator::new(&oa, &ob);
        merge.map(|x| resolve(x)).collect()
    }

    #[test]
    fn simple_case() {
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
}
