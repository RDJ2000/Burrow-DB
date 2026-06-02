use burrow_db::BurrowDB;
use std::io;

fn main() {
    let db= BurrowDB::new();
    println!("Hello, world from BurrowDB!");

    loop{
        let mut input=String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        let input= input.trim();

        

    }
}
