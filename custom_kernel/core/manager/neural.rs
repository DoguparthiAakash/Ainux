use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use crate::semantic::core::Sense;

pub struct NeuralNode {
    pub name: String,
    pub importance: f32, // 0.0 to 1.0
    pub tags: Vec<Sense>,
}

pub struct NeuralCortex {
    pub nodes: Vec<NeuralNode>,
}

pub static CORTEX: Mutex<NeuralCortex> = Mutex::new(NeuralCortex {
    nodes: Vec::new(),
});

impl NeuralCortex {
    pub fn associate(&mut self, name: &str, importance: f32, tags: Vec<Sense>) {
        self.nodes.push(NeuralNode {
            name: String::from(name),
            importance,
            tags,
        });
    }

    pub fn query_by_importance(&self, threshold: f32) -> Vec<String> {
        self.nodes.iter()
            .filter(|n| n.importance >= threshold)
            .map(|n| n.name.clone())
            .collect()
    }
}

pub fn init() {
    let mut cortex = CORTEX.lock();
    cortex.associate("Kernel Core", 1.0, alloc::vec![
        Sense { key: String::from("Type"), value: String::from("Critical") }
    ]);
    cortex.associate("Network Stack", 0.8, alloc::vec![
        Sense { key: String::from("Type"), value: String::from("Infrastructure") }
    ]);
}
