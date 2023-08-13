use std::collections::HashMap;
use bril_rs::{Function, Code, EffectOps, Instruction};
use petgraph::{Directed, Graph};

use crate::basic_block::BasicBlock;

type Label = String;

#[derive(Debug, Clone, Default)]
pub struct Cfg {
    blocks: HashMap<Label, BasicBlock>,
    graph: Graph<Label, (), Directed, u32>,
}

impl Cfg {
    pub fn from_function(func: Function) -> Self {
        let mut cfg = Self::default();

        let mut start_with_inst = 0;
        let entry_label = if let Some(Code::Label { label, .. }) = func.instrs.get(0) {
            start_with_inst = 1;
            label.to_owned()
        } else {
            "entry".to_string()
        };

        let entry_block = BasicBlock {
            node: cfg.graph.add_node(entry_label.clone()),
            label: entry_label.clone(),
            insts: Vec::new(),
        };
        cfg.blocks.insert(entry_label.clone(), entry_block);

        let mut cur_label = entry_label;
        for inst in func.instrs[start_with_inst..].iter() {
            if let Code::Instruction(inst) = inst {
                cfg.blocks.get_mut(&cur_label).unwrap().insts.push(inst.clone());
            }

            match inst {
                Code::Label{label, ..} => {
                    cfg.blocks.entry(label.clone()).or_insert_with( ||
                        BasicBlock {
                            node: cfg.graph.add_node(label.clone()),
                            label: label.clone(),
                            insts: Vec::new(),
                        },
                    );
                    cur_label = label.clone();
                } 
                Code::Instruction(Instruction::Effect { op, labels, .. }) => {
                    match op {
                        EffectOps::Branch => {
                            let left_label = &labels[0];
                            let left_node = cfg.blocks.entry(left_label.clone()).or_insert_with( ||
                                BasicBlock {
                                    node: cfg.graph.add_node(left_label.clone()),
                                    label: left_label.clone(),
                                    insts: Vec::new(),
                                }
                            ).node;

                            let right_label = &labels[1];
                            let right_node = cfg.blocks.entry(right_label.clone()).or_insert_with( ||
                                BasicBlock {
                                    node: cfg.graph.add_node(right_label.clone()),
                                    label: right_label.clone(),
                                    insts: Vec::new(),
                                }
                            ).node;

                            let cur_node = cfg.blocks.get(&cur_label).unwrap().node;
                            cfg.graph.add_edge(cur_node, left_node, ());
                            cfg.graph.add_edge(cur_node, right_node, ());
                        }
                        EffectOps::Jump => {
                            let target_label = &labels[0];
                            let target_node = cfg.blocks.entry(target_label.clone()).or_insert_with(|| 
                                BasicBlock {
                                    node: cfg.graph.add_node(target_label.clone()),
                                    label: target_label.clone(),
                                    insts: Vec::new(),
                                }
                            ).node;

                            let cur_node = cfg.blocks.get(&cur_label).unwrap().node;
                            cfg.graph.add_edge(cur_node, target_node, ());
                        },
                        // EffectOps::Return => {},
                        _ => {}
                    }
                }
                _ => {}
            }
        }

    cfg
    }
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

