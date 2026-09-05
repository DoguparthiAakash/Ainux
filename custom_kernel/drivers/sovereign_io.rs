use crate::object::KernelObject;
use crate::object::ObjectSnapshot;
use alloc::string::String;

#[derive(Debug)]
pub struct HardwareObject {
    pub name: String,
    pub port_start: u16,
    pub port_end: u16,
}

impl KernelObject for HardwareObject {
    fn name(&self) -> String { self.name.clone() }
    fn id(&self) -> usize { (self.port_start as usize) << 16 | (self.port_end as usize) }
    fn object_type(&self) -> &'static str { "HardwarePort" }

    fn snapshot(&self) -> Result<ObjectSnapshot, &'static str> {
        Ok(ObjectSnapshot { data: alloc::vec![0], related_handles: alloc::vec![] })
    }

    fn restore(&self, _snapshot: ObjectSnapshot) -> Result<(), &'static str> {
        Ok(())
    }
}

impl HardwareObject {
    pub fn new(name: &str, start: u16, end: u16) -> Self {
        Self {
            name: String::from(name),
            port_start: start,
            port_end: end,
        }
    }

    pub fn contains(&self, port: u16) -> bool {
        port >= self.port_start && port <= self.port_end
    }
}
