// error-pattern:found rust type
#[deny(ctypes)];

#[nolink]
extern mod libc {
    #[legacy_exports];
    fn malloc(size: int) -> *u8;
}

fn main() {
}