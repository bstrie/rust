#[deny(non_camel_case_types)];
/*!
 * Library to interface with chunks of memory allocated in C.
 *
 * It is often desirable to safely interface with memory allocated from C,
 * encapsulating the unsafety into allocation and destruction time.  Indeed,
 * allocating memory externally is currently the only way to give Rust shared
 * mut state with C programs that keep their own references; vectors are
 * unsuitable because they could be reallocated or moved at any time, and
 * importing C memory into a vector takes a one-time snapshot of the memory.
 *
 * This module simplifies the usage of such external blocks of memory.  Memory
 * is encapsulated into an opaque object after creation; the lifecycle of the
 * memory can be optionally managed by Rust, if an appropriate destructor
 * closure is provided.  Safety is ensured by bounds-checking accesses, which
 * are marshalled through get and set functions.
 *
 * There are three unsafe functions: the two introduction forms, and the
 * pointer elimination form.  The introduction forms are unsafe for the
 * obvious reason (they act on a pointer that cannot be checked inside the
 * method), but the elimination form is somewhat more subtle in its unsafety.
 * By using a pointer taken from a c_vec::t without keeping a reference to the
 * c_vec::t itself around, the CVec could be garbage collected, and the
 * memory within could be destroyed.  There are legitimate uses for the
 * pointer elimination form -- for instance, to pass memory back into C -- but
 * great care must be taken to ensure that a reference to the c_vec::t is
 * still held if needed.
 */

export CVec;
export c_vec, c_vec_with_dtor;
export get, set;
export len;
export ptr;

/**
 * The type representing a foreign chunk of memory
 *
 * Wrapped in a enum for opacity; FIXME #818 when it is possible to have
 * truly opaque types, this should be revisited.
 */
enum CVec<T> {
    CVec_({ base: *mut T, len: uint, rsrc: @DtorRes})
}

struct DtorRes {
  let dtor: option<fn@()>;
  new(dtor: option<fn@()>) { self.dtor = dtor; }
  drop {
    match self.dtor {
      option::none => (),
      option::some(f) => f()
    }
  }
}

/*
 Section: Introduction forms
 */

/**
 * Create a `CVec` from a foreign buffer with a given length.
 *
 * # Arguments
 *
 * * base - A foreign pointer to a buffer
 * * len - The number of elements in the buffer
 */
unsafe fn c_vec<T>(base: *mut T, len: uint) -> CVec<T> {
    return CVec_({
        base: base,
        len: len,
        rsrc: @DtorRes(option::none)
    });
}

/**
 * Create a `CVec` from a foreign buffer, with a given length,
 * and a function to run upon destruction.
 *
 * # Arguments
 *
 * * base - A foreign pointer to a buffer
 * * len - The number of elements in the buffer
 * * dtor - A function to run when the value is destructed, useful
 *          for freeing the buffer, etc.
 */
unsafe fn c_vec_with_dtor<T>(base: *mut T, len: uint, dtor: fn@())
  -> CVec<T> {
    return CVec_({
        base: base,
        len: len,
        rsrc: @DtorRes(option::some(dtor))
    });
}

/*
 Section: Operations
 */

/**
 * Retrieves an element at a given index
 *
 * Fails if `ofs` is greater or equal to the length of the vector
 */
fn get<T: copy>(t: CVec<T>, ofs: uint) -> T {
    assert ofs < len(t);
    return unsafe { *ptr::mut_offset((*t).base, ofs) };
}

/**
 * Sets the value of an element at a given index
 *
 * Fails if `ofs` is greater or equal to the length of the vector
 */
fn set<T: copy>(t: CVec<T>, ofs: uint, v: T) {
    assert ofs < len(t);
    unsafe { *ptr::mut_offset((*t).base, ofs) = v };
}

/*
 Section: Elimination forms
 */

/// Returns the length of the vector
fn len<T>(t: CVec<T>) -> uint {
    return (*t).len;
}

/// Returns a pointer to the first element of the vector
unsafe fn ptr<T>(t: CVec<T>) -> *mut T {
    return (*t).base;
}

#[cfg(test)]
mod tests {
    import libc::*;

    fn malloc(n: size_t) -> CVec<u8> {
        let mem = libc::malloc(n);

        assert mem as int != 0;

        return unsafe { c_vec_with_dtor(mem as *mut u8, n as uint,
                                     ||free(mem)) };
    }

    #[test]
    fn test_basic() {
        let cv = malloc(16u as size_t);

        set(cv, 3u, 8u8);
        set(cv, 4u, 9u8);
        assert get(cv, 3u) == 8u8;
        assert get(cv, 4u) == 9u8;
        assert len(cv) == 16u;
    }

    #[test]
    #[should_fail]
    #[ignore(cfg(windows))]
    fn test_overrun_get() {
        let cv = malloc(16u as size_t);

        get(cv, 17u);
    }

    #[test]
    #[should_fail]
    #[ignore(cfg(windows))]
    fn test_overrun_set() {
        let cv = malloc(16u as size_t);

        set(cv, 17u, 0u8);
    }

    #[test]
    fn test_and_I_mean_it() {
        let cv = malloc(16u as size_t);
        let p = unsafe { ptr(cv) };

        set(cv, 0u, 32u8);
        set(cv, 1u, 33u8);
        assert unsafe { *p } == 32u8;
        set(cv, 2u, 34u8); /* safety */
    }

}
