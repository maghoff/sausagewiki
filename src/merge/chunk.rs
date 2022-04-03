use std::fmt::Debug;

#[derive(Debug, PartialEq)]
pub struct Chunk<'a, Item: 'a + Debug + PartialEq + Copy>(
    pub &'a [diff::Result<Item>],
    pub &'a [diff::Result<Item>],
);
