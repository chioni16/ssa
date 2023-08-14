use bril_rs::Instruction;
use petgraph::graph::NodeIndex;

#[derive(Debug, Clone, Default)]
pub struct BasicBlock {
    pub node: NodeIndex,
    pub label: String,
    pub insts: Vec<Instruction>,
}

impl std::fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} ({}):", self.label, self.node.index())?;
        for inst in &self.insts {
            writeln!(f, "\t{}", inst)?;
        }

        Ok(())
    }
}
