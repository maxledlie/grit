/*
This is a command line utility for comparing the output of the Grit binary to that of Git.
*/
use clap::Parser;
use std::{fs, path::PathBuf, process::Command, env};
use anyhow::{Result, bail, anyhow};


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

    let left_exe = PathBuf::from(&args.left_exe).canonicalize()
        .map_err(|_| anyhow!("Could not find executable {}", &args.left_exe))?;

    let right_exe = PathBuf::from(&args.right_exe).canonicalize()
        .map_err(|_| anyhow!("Could not find executable {}", &args.right_exe))?;

    for entry in fs::read_dir(test_root)? {
        let entry = entry?;
        let path = entry.path().canonicalize()?;
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

            // Always run the Grit command in Git compatibility mode for tests
            let mut left_cmd_tokens = cmd_tokens.clone();
            left_cmd_tokens.push("-g");
            println!("Running left cmd");
            let left_output = Command::new(&left_exe)
                .args(&left_cmd_tokens)
                .output()
                .unwrap();

            if env::set_current_dir(&after_right).is_err() {
                bail!("Failed to set current dir to {}", after_right.to_string_lossy());
            }
            println!("Running right cmd");
            let right_output = Command::new(&right_exe)
                .args(&cmd_tokens)
                .output()
                .unwrap();


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

            println!("Comparing output contents");
            compare_directory(&after_left, &after_right, &test_name)?;

            // CLEANUP
            if !args.no_clean {
                fs::remove_dir_all(&after_left)?;
                fs::remove_dir_all(&after_right)?;
            }
        }
    }

    Ok(())
}

fn compare_directory(left: &PathBuf, right: &PathBuf, test_name: &str) -> Result<bool> {
    // Check that files in the left directory also exist in the right directory and
    // that their contents are exactly equal.
    for entry in fs::read_dir(left)? {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let right_entry = right.join(&file_name);
        if !right_entry.exists() {
            println!("Test {} fail", test_name);
            println!( "File {} exists in left output dir but not in right", &file_name);
            return Ok(false);
        }
        if entry.file_type()?.is_dir() {
            let left_child = entry.path().canonicalize()?;
            let right_child = right_entry.canonicalize()?;
            if !compare_directory(&left_child, &right_child, test_name)? {
                return Ok(false);
            }
        }
        if entry.file_type()?.is_file() {
            let left_bytes = fs::read(entry.path())?;
            let right_bytes = fs::read(right_entry)?;

            if left_bytes.len() != right_bytes.len() {
                println!("Test {} fail", test_name);
                println!("Mismatched sizes for {} in left and right output dirs", &file_name);
                return Ok(false);
            }

            for i in 0..left_bytes.len() {
                if left_bytes[i] != right_bytes[i] {
                    println!("Test {} fail", test_name);
                    println!("Mismatch in file {}", file_name);
                    return Ok(false);
                }
            }
        }
    }

    // Check that files in the right directory also exist in the left directory
    for entry in fs::read_dir(right)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if entry.file_type()?.is_file() {
            let left_entry = left.join(&file_name);
            if !left_entry.exists() {
                println!("Test {} fail", test_name);
                println!(
                    "File {} exists in right output dir but not in left",
                    file_name.to_string_lossy()
                );
                return Ok(false);
            }
        }
    }

    Ok(true)
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