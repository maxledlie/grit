use clap::Parser;

use grit::{Cli,
    Command,
    cmd_add,
    cmd_init,
    cmd_hash_object,
    cmd_cat_file,
    cmd_log,
    cmd_ls_files,
    cmd_checkout,
    cmd_status
};

fn main() {
    let args = Cli::parse();
    let global_opts = args.global_opts;

    let result = match args.command {
        Command::Add(args) => cmd_add(args, global_opts),
        Command::Init { path } => cmd_init(path, global_opts),
        Command::HashObject(args) => cmd_hash_object(args, global_opts),
        Command::CatFile(args) => cmd_cat_file(args, global_opts),
        Command::Log(args) => cmd_log(args, global_opts),
        Command::LsFiles(args) => cmd_ls_files(args, global_opts),
        Command::Checkout(args) => cmd_checkout(args, global_opts),
        Command::Status(args) => cmd_status(args, global_opts)
    };

    if let Some(err) = result.err() {
        eprintln!("{}", err);
    }
}
