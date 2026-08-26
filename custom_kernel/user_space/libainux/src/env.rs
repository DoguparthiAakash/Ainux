pub static mut ARGC: isize = 0;
pub static mut ARGV: *const *const u8 = core::ptr::null();

pub fn init(argc: isize, argv: *const *const u8) {
    unsafe {
        ARGC = argc;
        ARGV = argv;
    }
}

pub fn args<'a>() -> Args<'a> {
    unsafe { Args { argc: ARGC, argv: ARGV, current: 0, _marker: core::marker::PhantomData } }
}

pub struct Args<'a> {
    argc: isize,
    argv: *const *const u8,
    current: isize,
    _marker: core::marker::PhantomData<&'a u8>,
}

impl<'a> Iterator for Args<'a> {
    type Item = &'a str;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.argc || self.argv.is_null() {
            return None;
        }
        unsafe {
            let ptr = *self.argv.offset(self.current);
            if ptr.is_null() {
                return None;
            }
            
            let mut len = 0;
            while *ptr.offset(len) != 0 {
                len += 1;
            }
            
            let slice = core::slice::from_raw_parts(ptr, len as usize);
            self.current += 1;
            core::str::from_utf8(slice).ok()
        }
    }
}
