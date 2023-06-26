/*
This is a command line utility for comparing the output of the Grit binary to that of Git.
*/
use clap::Parser;
use std::{fs, path::PathBuf, process::{Command, Output}, env};
use anyhow::{Result, bail};


#[derive(Parser, Debug)]
#[command(author, version, about = "Pedant: a command line application for comparing the output of command line applications.")]
struct Args {
    #[arg(long)]
    no_clean: bool,
    test_dir: String,
    left_exe: String,
    right_exe: String
}

fn main() {
    let args = Args::parse();
    let result = run(args);

    if let Err(e) = result {
        println!("Error: {}", e.to_string());
    }
}

fn run(args: Args) -> Result<()> {
    let test_root = PathBuf::from(args.test_dir).canonicalize()?;
    if !test_root.exists() {
        bail!("Provided test root {} does not exist", test_root.to_string_lossy());
    }
    for entry in fs::read_dir(test_root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let default_name = String::from("???");
            let test_name = path.file_name().map(|x| x.to_string_lossy()).unwrap_or(default_name.into());

            // Copy the "before" directory into working directories for the left and right commands
            let before_dir = path.join("before");
            if !before_dir.exists() {
                println!("WARN: Test {} does not have a 'before' directory", test_name);
            }

            let after_left = path.join("after_left");
            let after_right = path.join("after_right");

            copy_dir(&before_dir, &after_left).unwrap();
            copy_dir(&before_dir, &after_right).unwrap();
            
            let cmd_path = path.join("cmds");
            let cmd_bytes = fs::read(cmd_path)?;
            let cmd_str = String::from_utf8_lossy(&cmd_bytes); 
            let cmd_tokens: Vec<&str> = cmd_str.split(" ").collect();


            if env::set_current_dir(&after_left).is_err() {
                bail!("Failed to set current dir to {}", after_left.to_string_lossy());
            }
            let left_exe = PathBuf::from(&args.left_exe);
            // Always run the Grit command in Git compatibility mode for tests
            let mut left_cmd_tokens = cmd_tokens.clone();
            left_cmd_tokens.push("-g");
            let left_output = Command::new(left_exe)
                .args(&left_cmd_tokens)
                .output()
                .unwrap();

            if env::set_current_dir(&after_right).is_err() {
                bail!("Failed to set current dir to {}", after_right.to_string_lossy());
            }
            let right_exe = PathBuf::from(&args.right_exe);
            let right_output = Command::new(&right_exe)
                .args(&cmd_tokens)
                .output()
                .unwrap();

            // CLEANUP
            if !args.no_clean {
                fs::remove_dir_all(after_left)?;
                fs::remove_dir_all(after_right)?;
            }

            // Replace references to test directory names in output
            let left_stdout = clean_output(&left_output.stdout, "after_left");
            let right_stdout = clean_output(&right_output.stdout, "after_right");
            let left_stderr = clean_output(&left_output.stderr, "after_left");
            let right_stderr = clean_output(&right_output.stderr, "after_right");

            if left_stdout != right_stdout {
                println!("Test {} fail", test_name);
                println!("stdout mismatch: expected");
                println!("{}", right_stdout);
                println!("but read:");
                println!("{}", left_stdout);
            }

            if left_stderr != right_stderr {
                println!("Test {} fail", test_name);
                println!("stderr mismatch: expected");
                println!("{}", right_stderr);
                println!("but read:");
                println!("{}", left_stderr);
            }
        }
    }

    Ok(())
}

fn clean_output(stdout: &Vec<u8>, dir_name: &str) -> String {
    String::from_utf8_lossy(stdout).replace(dir_name, "<dir_name>").trim().to_string()
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