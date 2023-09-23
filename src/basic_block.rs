use bril_rs::{Instruction, Type};
use petgraph::graph::NodeIndex;

#[derive(Debug, Clone, Default)]
pub struct BasicBlock {
    pub node: NodeIndex,
    pub label: String,
    pub insts: Vec<Instruction>,
    pub definitions: Vec<(String, Type)>,
}

impl BasicBlock {
    pub fn has_definition(&self, def: &(String, Type)) -> bool {
        self.definitions.contains(def)
    }
}

impl std::fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} ({}):", self.label, self.node.index())?;
        for inst in &self.insts {
            writeln!(f, "\t{}", inst)?;
        }

        // writeln!(f, "Definitions:")?;
        // for def in &self.definitions {
        //     writeln!(f, "{:?}", def)?;
        // }

        Ok(())
    }
}
