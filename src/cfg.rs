use bril_rs::{Code, EffectOps, Function, Instruction};
use petgraph::{
    graph::NodeIndex,
    visit::{Dfs, EdgeRef},
    Directed, Direction, Graph,
};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::basic_block::BasicBlock;
use crate::utils::{get_new_block, graph_to_svg};

type Label = String;

#[derive(Debug, Clone, Default)]
pub struct Cfg {
    entry_label: Label,
    blocks: HashMap<Label, BasicBlock>,
    graph: Graph<Label, (), Directed, u32>,
}

#[allow(dead_code)]
impl Cfg {
    pub fn remove_unreachable(&mut self) {
        let mut dfs = Dfs::new(&self.graph, self.blocks[&self.entry_label].node);
        let mut reachable = Vec::with_capacity(self.graph.node_count());
        while let Some(ni) = dfs.next(&self.graph) {
            reachable.push(ni);
        }

        let mut unreachable = self
            .graph
            .node_indices()
            .filter(|ni| !reachable.contains(ni))
            .collect::<Vec<_>>();
        unreachable.sort();
        unreachable.reverse();
        for ni in unreachable {
            self.graph.remove_node(ni);
        }
    }

    pub fn get_dominator_tree(&self) -> HashMap<Label, Option<Label>> {
        let entry_node = self.blocks[&self.entry_label].node;
        let all_nodes: HashSet<NodeIndex> = self.graph.node_indices().collect();
        let mut doms: HashMap<NodeIndex, HashSet<NodeIndex>> = self
            .graph
            .node_indices()
            .map(|ni| (ni, all_nodes.clone()))
            .collect();
        *doms.get_mut(&entry_node).unwrap() = HashSet::from_iter([entry_node]);

        let mut work_queue: VecDeque<NodeIndex> = VecDeque::from_iter(self.graph.node_indices());
        while let Some(el_node) = work_queue.pop_front() {
            let mut preds = self
                .graph
                .edges_directed(el_node, Direction::Incoming)
                .map(|edge| edge.source())
                .map(|ni| doms[&ni].clone())
                .fold(doms[&el_node].clone(), |acc, el| {
                    acc.intersection(&el).cloned().collect()
                });
            preds.insert(el_node);
            if preds != *doms.get(&el_node).unwrap() {
                *doms.get_mut(&el_node).unwrap() = preds;
                let outgoing_edges = self
                    .graph
                    .edges_directed(el_node, Direction::Outgoing)
                    .map(|edge| edge.target());
                work_queue.extend(outgoing_edges);
            }
        }

        let mut sdoms: HashMap<_, _> = doms
            .into_iter()
            .map(|(key, mut val)| {
                val.remove(&key);
                (key, val)
            })
            .collect();

        let mut idoms: HashMap<NodeIndex, Option<NodeIndex>> = HashMap::new();
        idoms.insert(entry_node, None);
        let mut work_queue = VecDeque::from_iter([entry_node]);
        while let Some(el_node) = work_queue.pop_front() {
            for (&node, set) in sdoms.iter_mut() {
                let present = set.remove(&el_node);
                if present && set.is_empty() {
                    idoms.insert(node, Some(el_node));
                    work_queue.push_back(node);
                }
            }
        }

        idoms
            .into_iter()
            .map(|(key, val)| {
                (
                    self.graph[key].clone(),
                    val.map(|val| self.graph[val].clone()),
                )
            })
            .collect()
    }

    pub fn output_graphviz(&self, filename: &str) {
        graph_to_svg(filename, &self.graph);
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
        let mut insts = func.instrs.iter().peekable();

        let entry_label = if let Some(Code::Label { label, .. }) = insts.peek() {
            insts.next();
            label.to_owned()
        } else {
            "entry".to_string()
        };

        let mut cfg = Cfg {
            entry_label: entry_label.clone(),
            ..Default::default()
        };

        self.add_new_block(&mut cfg, Some(&entry_label), true);

        while let Some(inst) = insts.next() {
            if let Code::Instruction(inst) = inst {
                cfg.blocks
                    .get_mut(&self.cur_label)
                    .unwrap()
                    .insts
                    .push(inst.clone());
            }

            match inst {
                Code::Label { label, .. } => {
                    self.add_new_block(&mut cfg, Some(label), true);
                }
                Code::Instruction(Instruction::Effect { op, labels, .. }) => match op {
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
                    }
                    EffectOps::Return => {
                        if let Some(Code::Instruction(_)) = insts.peek() {
                            self.add_new_block(&mut cfg, None, true);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        cfg
    }

    fn add_new_block<'a>(
        &'a mut self,
        cfg: &'a mut Cfg,
        label: Option<&String>,
        switch_to_new_block: bool,
    ) -> &mut BasicBlock {
        let label = label.cloned().unwrap_or_else(|| get_new_block());
        if switch_to_new_block {
            self.cur_label = label.clone();
        }
        cfg.blocks
            .entry(label.clone())
            .or_insert_with(|| BasicBlock {
                node: cfg.graph.add_node(label.clone()),
                label,
                insts: Vec::new(),
            })
    }

    fn add_new_edge_from_cur_block(&mut self, cfg: &mut Cfg, dest: &String) {
        let dest_node = self.add_new_block(cfg, Some(dest), false).node;
        let src_node = cfg.blocks.get(&self.cur_label).unwrap().node;
        cfg.graph.add_edge(src_node, dest_node, ());
    }
}
