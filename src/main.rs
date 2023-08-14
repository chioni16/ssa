mod basic_block;
mod cfg;
mod utils;

use bril_rs::load_program;
use cfg::CfgBuilder;

fn main() {
    let program = load_program();
    for func in program.functions {
        let mut builder = CfgBuilder::new();
        let mut cfg = builder.build(func);
        cfg.remove_unreachable();
        let dom_tree = cfg.get_dominator_tree();
        println!("{:?}", dom_tree);
    }
}
