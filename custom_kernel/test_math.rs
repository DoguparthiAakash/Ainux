fn main() {
    let height: i32 = 350;
    let width: i32 = 500;
    let mut max_idx = 0;
    for dy in 0..height {
        for dx in 0..width {
            let idx = (dy * width + dx) as usize;
            if idx > max_idx { max_idx = idx; }
        }
    }
    println!("Max idx: {}", max_idx);
}
