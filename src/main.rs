use clap::Parser;
use grit::{Cli, Command, cmd_init, cmd_hash_object, cmd_cat_file, cmd_log};


fn main() {
    let args = Cli::parse();
    let global_opts = args.global_opts;

    match args.command {
        Command::Init { path } => cmd_init(path, global_opts),
        Command::HashObject(args) => cmd_hash_object(args, global_opts),
        Command::CatFile(args) => cmd_cat_file(args, global_opts),
        Command::Log(args) => cmd_log(args, global_opts)
    }
}
