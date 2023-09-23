use bril_rs::{Code, EffectOps, Function, Instruction, Type, ValueOps};
use petgraph::{
    graph::NodeIndex,
    visit::{Dfs, EdgeRef},
    Directed,
    Direction::{self, Outgoing},
    Graph,
};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::utils::{get_new_block, graph_to_svg};
use crate::{basic_block::BasicBlock, utils};

type Label = String;

type IDoms = HashMap<NodeIndex, Option<NodeIndex>>;
type DominanceFrontiers = HashMap<NodeIndex, HashSet<NodeIndex>>;

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

    pub fn get_idoms(&self) -> IDoms {
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

        // idoms
        //     .into_iter()
        //     .map(|(key, val)| {
        //         (
        //             self.graph[key].clone(),
        //             val.map(|val| self.graph[val].clone()),
        //         )
        //     })
        //     .collect()
        idoms
    }

    pub fn get_dominance_frontiers(&mut self, idoms: IDoms) -> DominanceFrontiers {
        let mut df: HashMap<NodeIndex, HashSet<NodeIndex>> = self
            .graph
            .node_indices()
            .map(|ni| (ni, HashSet::new()))
            .collect();
        for node in self.graph.node_indices() {
            let predecessors: Vec<_> = self
                .graph
                .edges_directed(node, Direction::Incoming)
                .map(|edge| edge.source())
                .collect();
            if predecessors.len() > 1 {
                for pred in predecessors {
                    let mut runner = Some(pred);
                    while let Some(inner) = runner && runner != idoms[&node] {
                        df.get_mut(&inner).unwrap().insert(node);
                        runner = idoms[&inner];
                    }
                }
            }
        }

        println!("df: {:?}", df);
        df
    }

    pub fn insert_phi_nodes(&mut self, df: DominanceFrontiers) {
        let all_defs: HashSet<(String, Type)> = self
            .graph
            .node_weights()
            .map(|label| HashSet::from_iter(self.blocks[label].definitions.clone()))
            .fold(HashSet::new(), |acc, el| acc.union(&el).cloned().collect());

        println!("all_defs: {:?}", all_defs);

        for def in &all_defs {
            let def_blocks: HashSet<NodeIndex> = self
                .graph
                .node_weights()
                .filter_map(|label: &String| {
                    let block = self.blocks.get(label).unwrap();
                    if block.has_definition(def) {
                        Some(block.node)
                    } else {
                        None
                    }
                })
                .collect();
            println!("{:?} : {:?}", def, def_blocks);
            let mut done_blocks: HashSet<NodeIndex> = HashSet::new();
            let mut work_queue: VecDeque<NodeIndex> = VecDeque::from_iter(def_blocks.clone());

            while let Some(block) = work_queue.pop_front() {
                for &b in df.get(&block).unwrap() {
                    if !done_blocks.contains(&b) {
                        let bl = &self.graph[b];
                        // let incoming_nodes = self
                        //     .graph
                        //     .edges_directed(b, Direction::Incoming)
                        //     .count();
                        let inst = Instruction::Value {
                            // args: vec![def.0.clone(); incoming_nodes],
                            args: vec![],
                            dest: def.0.clone(),
                            funcs: vec![],
                            labels: vec![],
                            op: ValueOps::Phi,
                            pos: None,
                            op_type: def.1.clone(),
                        };
                        self.blocks.get_mut(bl).unwrap().insts.insert(0, inst);
                        done_blocks.insert(b);
                        println!("{:?} ==> {:?}", def, b);
                        if !def_blocks.contains(&b) {
                            work_queue.push_back(b);
                        }
                    }
                }
            }
        }
    }

    pub fn rename_variables(&mut self, idoms: IDoms) {
        let all_defs: HashSet<(String, Type)> = self
            .graph
            .node_weights()
            .map(|label| HashSet::from_iter(self.blocks[label].definitions.clone()))
            .fold(HashSet::new(), |acc, el| acc.union(&el).cloned().collect());

        let mut reaching_variables: HashMap<String, usize> =
            all_defs.iter().map(|(var, _)| (var.clone(), 0)).collect();

        let mut dom_tree: HashMap<NodeIndex, HashSet<NodeIndex>> = HashMap::new();
        let mut root = NodeIndex::default();
        for (child, parent) in idoms.into_iter() {
            if let Some(parent) = parent {
                let children = dom_tree.entry(parent).or_insert(HashSet::new());
                children.insert(child);
            } else {
                root = child;
            }
        }

        println!("reaching_variables: {:?}", reaching_variables);
        println!("dom_tree: {:?}", dom_tree);
        println!("root: {:?}", root);

        let mut work_queue: VecDeque<NodeIndex> = VecDeque::from_iter([root]);
        while let Some(block) = work_queue.pop_front() {
            println!("{:?}", block);
            let bl = &self.graph[block];
            for inst in &mut self.blocks.get_mut(bl).unwrap().insts {
                if !matches!(
                    inst,
                    Instruction::Value {
                        op: ValueOps::Phi,
                        ..
                    }
                ) {
                    match inst {
                        Instruction::Value { args, .. } | Instruction::Effect { args, .. } => {
                            for arg in args {
                                let version = reaching_variables.get(arg).copied().unwrap();
                                *arg = format!("{}.{}", arg, version);
                            }
                        }
                        _ => {}
                    }
                }

                match inst {
                    Instruction::Constant { dest, .. } | Instruction::Value { dest, .. } => {
                        let version = reaching_variables.get(dest).copied().unwrap();
                        *dest = format!("{}.{}", dest, version);
                        let dest = utils::extract_first_part(dest);
                        let _ = reaching_variables.insert(dest.to_string(), version + 1);
                    }
                    _ => {}
                }
            }

            for succ in self
                .graph
                .edges_directed(block, Outgoing)
                .map(|edge| edge.target())
            {
                let sbl = &self.graph[succ];
                for inst in &mut self.blocks.get_mut(sbl).unwrap().insts {
                    if let Instruction::Value {
                        op: ValueOps::Phi,
                        dest,
                        args,
                        labels,
                        ..
                    } = inst
                    {
                        let dest = utils::extract_first_part(dest);
                        let version = reaching_variables.get(dest).copied().unwrap();
                        args.push(format!("{dest}.{version}"));
                        labels.push(bl.clone());
                    }
                }
            }

            for &child in dom_tree.get(&block).unwrap_or(&HashSet::new()) {
                work_queue.push_back(child);
            }
        }
    }

    pub fn output_graphviz(&self, filename: &str) {
        graph_to_svg(filename, &self.graph);
    }
}

impl std::fmt::Display for Cfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for block in self.blocks.values() {
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
                let block = cfg.blocks.get_mut(&self.cur_label).unwrap();
                block.insts.push(inst.clone());
                match inst {
                    Instruction::Constant {
                        dest,
                        const_type: r#type,
                        ..
                    }
                    | Instruction::Value {
                        dest,
                        op_type: r#type,
                        ..
                    } => {
                        block.definitions.push((dest.clone(), r#type.clone()));
                    }
                    _ => {}
                }
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
        let label = label.cloned().unwrap_or_else(get_new_block);
        if switch_to_new_block {
            self.cur_label = label.clone();
        }
        cfg.blocks
            .entry(label.clone())
            .or_insert_with(|| BasicBlock {
                node: cfg.graph.add_node(label.clone()),
                label,
                insts: Vec::new(),
                definitions: Vec::new(),
            })
    }

    fn add_new_edge_from_cur_block(&mut self, cfg: &mut Cfg, dest: &String) {
        let dest_node = self.add_new_block(cfg, Some(dest), false).node;
        let src_node = cfg.blocks.get(&self.cur_label).unwrap().node;
        cfg.graph.add_edge(src_node, dest_node, ());
    }
}
