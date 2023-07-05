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
    println!("Running Pedant tests");
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

            if after_left.exists() {
                fs::remove_dir_all(&after_left)?;
            }
            if after_right.exists() {
                fs::remove_dir_all(&after_right)?;
            }

            copy_dir(&before_dir, &after_left).unwrap();
            copy_dir(&before_dir, &after_right).unwrap();
            
            let cmd_path = path.join("cmds");
            let cmd_bytes = fs::read(cmd_path)?;
            let cmd_str = String::from_utf8_lossy(&cmd_bytes); 
            let cmd_lines: Vec<&str> = cmd_str.split("\n").collect();

            let mut left_stdout = String::new();
            let mut left_stderr = String::new();
            let mut right_stdout = String::new();
            let mut right_stderr = String::new();
            
            // Run left command
            if env::set_current_dir(&after_left).is_err() {
                bail!("Failed to set current dir to {}", after_left.to_string_lossy());
            }
            for cmd_line in &cmd_lines {
                // Always run the Grit command in Git compatibility mode for tests
                let mut cmd_tokens: Vec<&str> = cmd_line.split(" ").collect();
                cmd_tokens.push("-g");
                let output = Command::new(&left_exe)
                    .args(&cmd_tokens)
                    .output()
                    .unwrap();

                left_stdout += &String::from_utf8_lossy(&output.stdout);
                left_stderr += &String::from_utf8_lossy(&output.stderr);
            }

            // Run right command
            if env::set_current_dir(&after_right).is_err() {
                bail!("Failed to set current dir to {}", after_right.to_string_lossy());
            }
            for cmd_line in &cmd_lines {
                let cmd_tokens: Vec<&str> = cmd_line.split(" ").collect();
                let output = Command::new(&right_exe)
                    .args(&cmd_tokens)
                    .output()
                    .unwrap();

                right_stdout += &String::from_utf8_lossy(&output.stdout);
                right_stderr += &String::from_utf8_lossy(&output.stderr);
            }

            // Replace references to test directory names in output
            let left_stdout = clean_output(left_stdout, "after_left");
            let right_stdout = clean_output(right_stdout, "after_right");
            let left_stderr = clean_output(left_stderr, "after_left");
            let right_stderr = clean_output(right_stderr, "after_right");

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

            // Run Unix diff command to print differences between left and right directories
            let diff_args = vec![
                after_left.to_string_lossy().to_string(),
                after_right.to_string_lossy().to_string(),
                String::from("--recursive"),
                String::from("--color"),
                String::from("--exclude-from"),
                String::from("../../exclude")
            ];
            let diff_output = Command::new("diff").args(diff_args).output().unwrap();

            if diff_output.stderr.len() > 0 || diff_output.stdout.len() > 0 {
                println!("Test {} failed:", &test_name);
                println!("{}", String::from_utf8_lossy(&diff_output.stderr));
                println!("{}", String::from_utf8_lossy(&diff_output.stdout));
            }

            // CLEANUP
            if !args.no_clean {
                fs::remove_dir_all(&after_left)?;
                fs::remove_dir_all(&after_right)?;
            }
        }
    }

    Ok(())
}

fn copy_dir(from: &PathBuf, to: &PathBuf) -> Result<()> {
    let args = vec![
        String::from("-r"),
        from.to_string_lossy().to_string(),
        to.to_string_lossy().to_string()
    ];
    let output = Command::new("cp").args(args).output()?;
    if output.stderr.len() > 0 { 
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if output.stdout.len() > 0 {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    Ok(())
}

fn clean_output(output: String, dir_name: &str) -> String {
    output.replace(dir_name, "<dir_name>").trim().to_string()
}