mod basic_block;
mod cfg;
mod utils;

use cfg::CfgBuilder;
use bril_rs::load_program;

fn main() {
    let program = load_program();
    for func in program.functions {
        let mut builder = CfgBuilder::new();
        let cfg = builder.build(func);
        println!("{}", cfg);
    }
}
