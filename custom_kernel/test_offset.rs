#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Context {
    pub rsp: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rip: u64,
}

#[repr(C, align(16))]
pub struct Task {
    pub id: usize,
    pub context: Context,
}

fn main() {
    let t = Task { id: 0, context: Context::default() };
    let base = &t as *const _ as usize;
    let rsp_off = &t.context.rsp as *const _ as usize - base;
    let r13_off = &t.context.r13 as *const _ as usize - base;
    println!("Context.rsp offset: {}", rsp_off);
    println!("Context.r13 offset: {}", r13_off);
}
