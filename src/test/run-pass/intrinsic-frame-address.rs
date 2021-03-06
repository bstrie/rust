#[abi = "rust-intrinsic"]
native mod rusti {
    fn frame_address(f: fn(*u8));
}

fn main() {
    rusti::frame_address {|addr|
        assert addr.is_not_null();
    }
}
