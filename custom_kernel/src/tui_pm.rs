use crate::drivers::video;
use crate::drivers::mbr::MbrPartition;
use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::keyboard;

// Colors (Catppuccin Macchiato inspired)
const COL_BORDER: u32 = 0x00585B70;
const COL_DISK: u32 = 0x008CAAEE; // Blue
const COL_PART: u32 = 0x00A6DA95; // Green
const COL_UNALLOC: u32 = 0x006C7086; // Overlay0
const COL_SWAP: u32 = 0x00ED8796; // Red
const COL_TEXT: u32 = 0x00CAD3F5; // Text
const COL_SEL_BG: u32 = 0x00CBA6F7; // Mauve
const COL_SEL_FG: u32 = 0x00181825; // Mantle (Dark)

#[derive(Clone)]
enum EntryType {
    Disk,
    Partition(usize, MbrPartition), // index, partition data
    Unallocated(u32, u32), // start_lba, num_sectors
}

struct PmState {
    capacity_sectors: u32,
    partitions: Vec<MbrPartition>,
    entries: Vec<(String, String, EntryType)>, 
    selected_index: usize,
    scroll_offset: usize,
    status_msg: String,
}

impl PmState {
    fn new() -> Self {
        let mut state = Self {
            capacity_sectors: crate::drivers::ata::get_drive_capacity(),
            partitions: Vec::new(),
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            status_msg: String::from("Ready."),
        };
        
        let mbr_parts = crate::drivers::mbr::parse_mbr();
        for p in mbr_parts {
            state.partitions.push(p);
        }
        
        state.rebuild_entries();
        state
    }

    fn rebuild_entries(&mut self) {
        self.entries.clear();
        self.partitions.sort_by_key(|p| p.lba_start);
        
        self.entries.push((
            String::from("Disk 0: hda (Primary Master)"), 
            String::from("DISK"),
            EntryType::Disk
        ));
        
        let mut current_lba = 2048; // Leave space for MBR & alignment
        
        for (i, p) in self.partitions.iter().enumerate() {
            if p.lba_start > current_lba {
                let gap = p.lba_start - current_lba;
                if gap > 2048 {
                    self.entries.push((
                        alloc::format!("  └─ Unallocated"),
                        String::from("FREE"),
                        EntryType::Unallocated(current_lba, gap)
                    ));
                }
            }
            
            let p_type_str = match p.partition_type {
                0x83 => "Linux/Ext2",
                0x82 => "Linux Swap",
                0x0B | 0x0C => "FAT32",
                0x07 => "NTFS/exFAT",
                _ => "Unknown",
            };
            
            self.entries.push((
                alloc::format!("  ├─ hda{} (Type: 0x{:02X})", i + 1, p.partition_type),
                String::from(p_type_str),
                EntryType::Partition(i, *p)
            ));
            
            current_lba = p.lba_start + p.num_sectors;
        }
        
        if self.capacity_sectors > current_lba {
            let gap = self.capacity_sectors - current_lba;
            if gap > 2048 {
                self.entries.push((
                    alloc::format!("  └─ Unallocated"),
                    String::from("FREE"),
                    EntryType::Unallocated(current_lba, gap)
                ));
            }
        }
        
        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let mut truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        truncated.push('~');
        truncated
    } else {
        String::from(s)
    }
}

fn draw_usage_bar(state: &PmState, y: usize, width: usize, bg: u32) {
    let bar_width = width.saturating_sub(4);
    if state.capacity_sectors == 0 || bar_width == 0 { return; }
    
    video::put_char_at(1, y, '[', COL_BORDER, bg);
    video::put_char_at(width - 2, y, ']', COL_BORDER, bg);
    
    let sectors_per_char = (state.capacity_sectors as f64 / bar_width as f64) as u32;
    if sectors_per_char == 0 { return; }
    
    let mut current_x = 2;
    for p in &state.partitions {
        let chars_to_draw = (p.num_sectors / sectors_per_char) as usize;
        let color = match p.partition_type {
            0x82 => COL_SWAP,
            0x0B | 0x0C | 0x07 => 0x00F9E2AF, // Yellow
            _ => COL_PART,
        };
        for _ in 0..chars_to_draw {
            if current_x < width - 2 {
                video::put_char_at(current_x, y, '█', color, bg);
                current_x += 1;
            }
        }
    }
    
    // Fill remaining with unallocated
    while current_x < width - 2 {
        video::put_char_at(current_x, y, '▒', COL_UNALLOC, bg);
        current_x += 1;
    }
}

pub fn launch_pm() {
    let mut state = PmState::new();
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            video::clear();
            let width = *video::CONSOLE_WIDTH.lock();
            let height = *video::CONSOLE_HEIGHT.lock();
            let bg = video::THEME.lock().bg;

            let left_w = (width * 45) / 100;
            let pane_h = height.saturating_sub(6);

            // Draw Top Header (Removed Emoji for font compatibility)
            video::put_str_at(0, 0, " Ainux Partition Manager ", COL_SEL_BG, bg);

            // Draw Usage Bar
            draw_usage_bar(&state, 2, width, bg);

            // Pane Separators
            for j in 4..(height - 2) {
                video::put_char_at(left_w, j, '│', COL_BORDER, bg);
            }

            // --- PANE 1: Device List ---
            for (i, (name, _, _)) in state.entries.iter().skip(state.scroll_offset).take(pane_h).enumerate() {
                let actual_idx = i + state.scroll_offset;
                let display_str = truncate(name, left_w.saturating_sub(2));
                
                let is_sel = actual_idx == state.selected_index;
                let (fg, cbg) = if is_sel {
                    (COL_SEL_FG, COL_SEL_BG)
                } else if name.starts_with("Disk") {
                    (COL_DISK, bg)
                } else if name.contains("Unallocated") {
                    (COL_UNALLOC, bg)
                } else {
                    (COL_PART, bg)
                };
                
                if is_sel {
                    for x in 0..left_w {
                        video::put_char_at(x, 4 + i, ' ', fg, cbg);
                    }
                }
                video::put_str_at(1, 4 + i, &display_str, fg, cbg);
            }

            // --- PANE 2: Details ---
            if !state.entries.is_empty() {
                let sel_idx = state.selected_index;
                let (name, type_str, entry_type) = &state.entries[sel_idx];
                
                video::put_str_at(left_w + 2, 4, "Information:", COL_TEXT, bg);
                video::put_str_at(left_w + 2, 5, &alloc::format!("Name: {}", name.trim()), COL_TEXT, bg);
                video::put_str_at(left_w + 2, 6, &alloc::format!("Type: {}", type_str), COL_TEXT, bg);
                
                match entry_type {
                    EntryType::Disk => {
                        let size_mb = (state.capacity_sectors as u64 * 512) / (1024 * 1024);
                        video::put_str_at(left_w + 2, 8, &alloc::format!("Total Size: {} MB", size_mb), COL_TEXT, bg);
                        video::put_str_at(left_w + 2, 9, "Bus: ATA (IDE)", COL_TEXT, bg);
                    },
                    EntryType::Partition(_, p) => {
                        let size_mb = (p.num_sectors as u64 * 512) / (1024 * 1024);
                        video::put_str_at(left_w + 2, 8, &alloc::format!("Start LBA: {}", p.lba_start), COL_TEXT, bg);
                        video::put_str_at(left_w + 2, 9, &alloc::format!("Sectors:   {}", p.num_sectors), COL_TEXT, bg);
                        video::put_str_at(left_w + 2, 10, &alloc::format!("Size:      {} MB", size_mb), COL_TEXT, bg);
                    },
                    EntryType::Unallocated(start, size) => {
                        let size_mb = (*size as u64 * 512) / (1024 * 1024);
                        video::put_str_at(left_w + 2, 8, &alloc::format!("Start LBA: {}", start), COL_TEXT, bg);
                        video::put_str_at(left_w + 2, 9, &alloc::format!("Size:      {} MB", size_mb), COL_TEXT, bg);
                    }
                }
            }

            // Draw Bottom Footer & Status (Usage Guide)
            let guide = " [W/S] Nav  [C]reate  [D]elete  [Shift+W]rite  [Q]uit | ";
            let status = alloc::format!("Status: {}", state.status_msg);
            let footer = alloc::format!("{}{}", guide, status);
            let padded_footer = alloc::format!("{:width$}", footer, width=width);
            video::put_str_at(0, height - 1, &truncate(&padded_footer, width), COL_SEL_BG, bg);

            needs_redraw = false;
        }

        let c = crate::drivers::keyboard::get_char();
        if c != '\0' {
            match c {
                'q' | 'Q' => {
                    video::clear();
                    return;
                },
                'w' | 'W' => {
                    if crate::drivers::mbr::write_mbr(&state.partitions) {
                        crate::drivers::partition::reload();
                        state.status_msg = String::from("MBR Written Successfully.");
                    } else {
                        state.status_msg = String::from("ERROR: Failed to write MBR.");
                    }
                    needs_redraw = true;
                },
                'd' | 'D' => {
                    let entry_type = state.entries[state.selected_index].2.clone();
                    if let EntryType::Partition(idx, _) = entry_type {
                        state.partitions.remove(idx);
                        state.rebuild_entries();
                        state.status_msg = String::from("Partition deleted from memory.");
                        needs_redraw = true;
                    } else {
                        state.status_msg = String::from("Cannot delete this entry.");
                        needs_redraw = true;
                    }
                },
                'c' | 'C' => {
                    let entry_type = state.entries[state.selected_index].2.clone();
                    if let EntryType::Unallocated(start, size) = entry_type {
                        if state.partitions.len() < 4 {
                            state.partitions.push(MbrPartition {
                                status: 0x80, // bootable by default
                                partition_type: 0x83, // Linux by default
                                lba_start: start,
                                num_sectors: size,
                            });
                            state.rebuild_entries();
                            state.status_msg = String::from("Created new partition.");
                        } else {
                            state.status_msg = String::from("Maximum 4 primary partitions reached.");
                        }
                        needs_redraw = true;
                    } else {
                        state.status_msg = String::from("Select unallocated space to create.");
                        needs_redraw = true;
                    }
                },
                'r' | 'R' => {
                    state = PmState::new();
                    needs_redraw = true;
                },
                'k' | 'w' => { // Up
                    if state.selected_index > 0 {
                        state.selected_index -= 1;
                        if state.selected_index < state.scroll_offset {
                            state.scroll_offset = state.selected_index;
                        }
                        needs_redraw = true;
                    }
                },
                'j' | 's' => { // Down
                    if state.selected_index < state.entries.len().saturating_sub(1) {
                        state.selected_index += 1;
                        let pane_h = *video::CONSOLE_HEIGHT.lock() - 6;
                        if state.selected_index >= state.scroll_offset + pane_h - 1 {
                            state.scroll_offset += 1;
                        }
                        needs_redraw = true;
                    }
                },
                _ => {}
            }
        }
        crate::process::scheduler::yield_now();
    }
}
