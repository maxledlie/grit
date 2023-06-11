use grit::{cmd_checkout, CheckoutArgs, GlobalOpts};
use utils::testbed;
use std::{fs};

#[test]
fn fails_if_directory_is_not_empty() {
    let tempdir = testbed();
    
    let path = tempdir.root.join("foo.txt");
    fs::write(path, "hello world").unwrap();

    let args = CheckoutArgs {
        commit: String::from("fake_hash"),
        directory: tempdir.root.to_string_lossy().to_string()
    };


    assert!(cmd_checkout(args, GlobalOpts { git_mode: false }).is_err());
}