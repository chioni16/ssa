mod basic_block;
mod cfg;
mod utils;

use cfg::Cfg;
use bril_rs::load_program;

fn main() {
    let program = load_program();
    for func in program.functions {
        let cfg = Cfg::from_function(func);
        println!("{}", cfg);
    }
}
