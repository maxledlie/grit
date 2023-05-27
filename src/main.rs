use clap::Parser;
use grit::{Cli, Commands, cmd_init, cmd_hash_object, cmd_cat_file, cmd_log};


fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Init { path } => cmd_init(path),
        Commands::HashObject(args) => cmd_hash_object(args),
        Commands::CatFile(args) => cmd_cat_file(args),
        Commands::Log(args) => cmd_log(args)
    }
}
