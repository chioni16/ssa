#![feature(let_chains)]

mod basic_block;
mod cfg;
mod optimisations;
mod utils;

use bril_rs::load_program;
use cfg::CfgBuilder;

fn main() {
    let program = load_program();
    for func in program.functions {
        let mut builder = CfgBuilder::new();
        let mut cfg = builder.build(func);
        cfg.remove_unreachable();
        println!("{cfg}");
        cfg.output_graphviz("/Users/ggd/projects/ssa/ihatemyself");
        println!("======================================================================================");
        let idoms = cfg.get_idoms();
        let df = cfg.get_dominance_frontiers(idoms.clone());
        println!("{:?}", df);
        println!("======================================================================================");
        cfg.insert_phi_nodes(df);
        cfg.rename_variables(idoms);
        println!("{cfg}");
    }
}
