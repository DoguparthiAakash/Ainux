use crate::drivers::mbr::MbrPartition;
use crate::drivers::ata;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug)]
pub struct LogicalPartition {
    pub id: usize,
    pub partition_type: u8,
    pub start_lba: u32,
    pub num_sectors: u32,
}

impl LogicalPartition {
    pub fn read_sectors(&self, target: &mut [u16], relative_lba: u32, sectors: u8) -> bool {
        if relative_lba + (sectors as u32) > self.num_sectors {
            return false;
        }
        let absolute_lba = self.start_lba + relative_lba;
        ata::read_sectors(target, absolute_lba, sectors)
    }

    pub fn write_sectors(&self, data: &[u16], relative_lba: u32, sectors: u8) -> bool {
        if relative_lba + (sectors as u32) > self.num_sectors {
            return false;
        }
        let absolute_lba = self.start_lba + relative_lba;
        ata::write_sectors(data, absolute_lba, sectors)
    }
}

lazy_static! {
    pub static ref PARTITIONS: Mutex<Vec<LogicalPartition>> = Mutex::new(Vec::new());
}

pub fn init() {
    let mbr_parts = crate::drivers::mbr::parse_mbr();
    let mut parts = PARTITIONS.lock();
    for (i, p) in mbr_parts.iter().enumerate() {
        parts.push(LogicalPartition {
            id: i,
            partition_type: p.partition_type,
            start_lba: p.lba_start,
            num_sectors: p.num_sectors,
        });
    }
}
