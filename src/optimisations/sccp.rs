use std::collections::{HashMap, HashSet, VecDeque};

use bril_rs::{EffectOps, Instruction, Literal, ValueOps};
use petgraph::{
    graph::{EdgeIndex, EdgeReference},
    visit::EdgeRef,
    Direction,
};

use crate::cfg::Cfg;

#[derive(Debug, Clone, PartialEq)]
enum SccpLattice {
    Top,
    Constant(Literal),
    Bottom,
}

fn meet(a: &SccpLattice, b: &SccpLattice) -> SccpLattice {
    if a == b {
        return a.clone();
    }
    match (a, b) {
        (a, SccpLattice::Top) => a.clone(),
        (SccpLattice::Top, b) => b.clone(),
        (_, _) => SccpLattice::Bottom,
    }
}

impl Cfg {
    pub fn sccp(&self) {
        let (ssa_graph, names) = self.ssa_graph();

        let entry_node = self.blocks[&self.entry_label].node;

        let mut cfg_work_queue =
            VecDeque::from_iter(self.graph.edges_directed(entry_node, Direction::Outgoing));
        let mut ssa_work_queue = VecDeque::<EdgeReference<()>>::new();

        let mut executable_cfg_edges = HashSet::new();
        let mut visited_blocks = HashSet::new();

        // Remove the top element of one of the two work lists
        loop {
            if let Some(edge) = cfg_work_queue.pop_front() {
                // Mark the edge as executable
                executable_cfg_edges.insert(edge.id());

                // Visit every φ-function associated with the target node
                let target_label = &self.graph[edge.target()];
                let target_block = &self.blocks[target_label];
                for inst in &target_block.insts {
                    if matches!(
                        inst,
                        Instruction::Value {
                            op: ValueOps::Phi,
                            ..
                        }
                    ) {
                        self.visit_phi();
                    }
                }

                // If the target node was reached the first time via the CFGWorkList, visit all its operations
                if !visited_blocks.contains(target_label) {
                    for inst in &target_block.insts {
                        self.visit_expr();
                    }
                    visited_blocks.insert(target_label);
                }

                // If the target node has a single, non-executable outgoing edge, append that edge to the CFGWorkList
                let mut edges = self
                    .graph
                    .edges_directed(edge.target(), Direction::Outgoing);
                if let Some(edge) = edges.next() && !executable_cfg_edges.contains(&edge.id()) && edges.next().is_none() {
                    cfg_work_queue.push_back(edge);
                }

                continue;
            }

            if let Some(edge) = ssa_work_queue.pop_front() {
                let (block, index) = ssa_graph[edge.target()];

                let target_block = &self.blocks[block];
                let target_inst = &target_block.insts[index];

                // When the target operation is a φ-function visit that φ-function
                if matches!(
                    target_inst,
                    Instruction::Value {
                        op: ValueOps::Phi,
                        ..
                    }
                ) {
                    self.visit_phi();
                } else {
                    // For other operations,
                    let a = self
                        .graph
                        .node_indices()
                        .find(|&ni| &self.graph[ni] == block)
                        .unwrap();
                    // examine the executable flag of the incoming edges of the respective CFG node
                    let b = self
                        .graph
                        .edges_directed(a, Direction::Incoming)
                        .any(|edge| executable_cfg_edges.contains(&edge.id()));
                    // Visit the operation if any of the edges is executable.
                    if b {
                        self.visit_expr();
                    }
                }

                continue;
            }

            // Continue until both work lists become empty.
            break;
        }
    }

    fn visit_phi(&self) {}

    fn visit_expr(&self) {}

    fn visit_inst<'cfg>(
        &'cfg self,
        curr_block_label: &String,
        inst_index: usize,
        executable_cfg_edges: &mut HashSet<EdgeIndex>,
        lattices: &mut HashMap<&'cfg String, SccpLattice>,
    ) {
        let inst = &self.blocks[curr_block_label].insts[inst_index];

        match inst {
            // φ-functions:
            // Combine the data-flow information from the node’s operands where the corresponding control-flow edge is executable.
            Instruction::Value {
                op: ValueOps::Phi,
                dest,
                args,
                labels,
                ..
            } => {
                let dest_block = self
                    .graph
                    .node_indices()
                    .find(|&ni| &self.graph[ni] == curr_block_label)
                    .unwrap();

                let lattice = args
                    .iter()
                    .zip(labels)
                    .filter_map(|(arg, source_label)| {
                        let source_block = self
                            .graph
                            .node_indices()
                            .find(|&ni| &self.graph[ni] == source_label)
                            .unwrap();

                        let edge = self.graph.find_edge(source_block, dest_block).unwrap();

                        if executable_cfg_edges.contains(&edge) {
                            Some(arg)
                        } else {
                            None
                        }
                    })
                    .fold(SccpLattice::Top, |acc, arg| {
                        // SccpLattice::Top is the default value
                        // None = SccpLattice::Top
                        let arg = lattices.get(arg).unwrap_or(&SccpLattice::Top);
                        meet(&acc, arg)
                    });

                lattices.insert(dest, lattice);
            }
            // Conditional branches:
            // Examine the branch’s condition(s) using the data-flow information of its operands;
            // Determine all outgoing edges of the branch’s CFG node whose condition is potentially
            // satisfied; Append the CFG edges that were non-executable to the CFGWorkList.
            Instruction::Effect {
                op: EffectOps::Branch,
                ..
            } => {}
            // Other operations
            // Update the operation’s data-flow information by applying its transfer function.
            Instruction::Constant { dest, value, .. } => {
                match lattices.get(dest) {
                    None => {
                        lattices.insert(dest, SccpLattice::Constant(value.clone()));
                    }
                    Some(SccpLattice::Constant(constant)) if constant == value => {}
                    _ => {
                        lattices.insert(dest, SccpLattice::Bottom);
                    }
                };
            }
            _ => {}
        }
    }
}
