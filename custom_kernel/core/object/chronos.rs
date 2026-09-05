use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;
use crate::object::ObjectSnapshot;

/// The Chronos Manager handles the temporal storage of object states.
pub struct ChronosVault {
    snapshots: BTreeMap<usize, ObjectSnapshot>,
    next_id: usize,
}

pub static VAULT: Mutex<ChronosVault> = Mutex::new(ChronosVault {
    snapshots: BTreeMap::new(),
    next_id: 1,
});

impl ChronosVault {
    pub fn save(&mut self, snapshot: ObjectSnapshot) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.snapshots.insert(id, snapshot);
        id
    }

    pub fn load(&mut self, id: usize) -> Option<ObjectSnapshot> {
        self.snapshots.remove(&id)
    }
}
