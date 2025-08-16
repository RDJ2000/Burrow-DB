use burrow_db::{BurrowDB, cli::CLI};

fn main() {
    let mut db = BurrowDB::new();
    let mut cli = CLI::new(&mut db);
    cli.run();
}
