// Issue #1763 - infer types correctly

type actor<T> = { //! ERROR type parameter `T` is unused
    unused: bool
};

fn main() {}
