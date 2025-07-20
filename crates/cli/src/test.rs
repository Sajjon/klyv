#![cfg(test)]

use std::env;
use test_log::test;

use crate::{Input, run};

#[test]
fn test() {
    let mut path = env::current_dir().unwrap();
    path.push("src/fixtures/fixt0");
    let input = Input::builder().path(path.to_path_buf()).build();
    let tree = run(input).unwrap();
    let debug = format!("{:#?}", tree);
    assert!(debug.contains("AaaaStructB"));
    assert!(debug.contains("global_gen_magic"));
    assert!(debug.contains("AbAStructA"));
}
