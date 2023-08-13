use std::collections::HashMap;
use bril_rs::{Function, Code, EffectOps, Instruction};
use petgraph::{Directed, Graph};

use crate::{basic_block::BasicBlock, utils::get_new_block};

type Label = String;

#[derive(Debug, Clone, Default)]
pub struct Cfg {
    blocks: HashMap<Label, BasicBlock>,
    graph: Graph<Label, (), Directed, u32>,
}

impl std::fmt::Display for Cfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (_, block) in &self.blocks {
            writeln!(f, "{block}")?;
        }

        for node in self.graph.node_indices() {
            let neighbours: Vec<_> = self.graph.neighbors(node).map(|n| &self.graph[n]).collect();
            writeln!(f, "{} -> {:?}", self.graph[node], neighbours)?;
        }

        Ok(())
    }
}

pub struct CfgBuilder {
    cur_label: String,
}

impl CfgBuilder {
    pub fn new() -> Self {
        Self {
            cur_label: "".to_string(),
        }
    }

    pub fn build(&mut self, func: Function) -> Cfg {
        let mut cfg = Cfg::default();

        let mut insts = func.instrs.iter().peekable();

        let entry_label = if let Some(Code::Label { label, .. }) = insts.peek() {
            insts.next();
            label.to_owned()
        } else {
            "entry".to_string()
        };

        self.add_new_block(&mut cfg, Some(&entry_label), true);

        while let Some(inst) = insts.next() {
            if let Code::Instruction(inst) = inst {
                cfg.blocks.get_mut(&self.cur_label).unwrap().insts.push(inst.clone());
            }

            match inst {
                Code::Label{label, ..} => {
                    self.add_new_block(&mut cfg, Some(label), true);
                } 
                Code::Instruction(Instruction::Effect { op, labels, .. }) => {
                    match op {
                        EffectOps::Branch => {
                            self.add_new_edge_from_cur_block(&mut cfg, &labels[0]);
                            self.add_new_edge_from_cur_block(&mut cfg, &labels[1]);

                            if let Some(Code::Instruction(_)) = insts.peek() {
                                self.add_new_block(&mut cfg, None, true);
                            }
                        }
                        EffectOps::Jump => {
                            self.add_new_edge_from_cur_block(&mut cfg, &labels[0]);

                            if let Some(Code::Instruction(_)) = insts.peek() {
                                self.add_new_block(&mut cfg, None, true);
                            }
                        },
                        EffectOps::Return => {
                            if let Some(Code::Instruction(_)) = insts.peek() {
                                self.add_new_block(&mut cfg, None, true);
                            }
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        cfg
    }

    fn add_new_block<'a>(&'a mut self, cfg: &'a mut Cfg, label: Option<&String>, switch_to_new_block: bool) -> &mut BasicBlock {
        let label = label.cloned().unwrap_or_else(|| get_new_block());
        if switch_to_new_block {
            self.cur_label = label.clone();
        }
        cfg.blocks.entry(label.clone()).or_insert_with( ||
            BasicBlock {
                node: cfg.graph.add_node(label.clone()),
                label,
                insts: Vec::new(),
            }
        )
    }

    fn add_new_edge_from_cur_block(&mut self, cfg: &mut Cfg, dest: &String) {
        let dest_node = self.add_new_block(cfg, Some(dest), false).node;
        let src_node = cfg.blocks.get(&self.cur_label).unwrap().node;
        cfg.graph.add_edge(src_node, dest_node, ());
    }
}
