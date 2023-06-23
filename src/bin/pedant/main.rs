/*
This is a command line utility for comparing the output of the Grit binary to that of Git.
*/
use clap::Parser;
use std::{fs, path::PathBuf, process::Command};
use anyhow::Result;


#[derive(Parser, Debug)]
#[command(author, version, about = "Pedant: a command line application for comparing the output of command line applications.")]
struct Args {
    test_dir: String,
}


fn main() {
    let args = Args::parse();
    let result = run(args);

    if let Err(e) = result {
        println!("Error: {}", e.to_string());
    }
}

fn run(args: Args) -> Result<()> {
    let test_root = PathBuf::from(args.test_dir);
    for entry in fs::read_dir(test_root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Copy the "before" directory into working directories for the left and right commands
            let before_dir = path.join("before");
            let after_left = path.join("after_left");
            let after_right = path.join("after_right");

            copy_dir(&before_dir, &after_left)?;
            copy_dir(&before_dir, &after_right)?;
            
            let default_name = String::from("???");
            let test_name = path.file_name().map(|x| x.to_string_lossy()).unwrap_or(default_name.into());
            println!("Running test {}", test_name);

            // Read the "left" command
            let left_cmd_path = path.join("cmd_left");
            let left_cmd_bytes = fs::read(left_cmd_path)?;
            let _left_cmd = String::from_utf8_lossy(&left_cmd_bytes); 

            // CLEANUP
            fs::remove_dir_all(after_left)?;
            fs::remove_dir_all(after_right)?;
        }
    }

    Ok(())
}

fn copy_dir(source: &PathBuf, target: &PathBuf) -> Result<()> {
    if !target.exists() {
        fs::create_dir(target)?;
    }

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // TODO: Copy recursively, not just files in the root
        } else {
            match path.file_name() {
                Some(filename) => {
                    let dest_path = target.join(filename);
                    fs::copy(&path, &dest_path)?;
                },
                None => {
                    println!("Failed to copy {:?}", path);
                }
            }
        }
    }

    Ok(())
}