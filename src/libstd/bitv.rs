#[deny(non_camel_case_types)];
import vec::{to_mut, from_elem};

export Bitv;
export union;
export Union;
export intersect;
export Intersect;
export assign;
export Assign;
export difference;
export Difference;
export clone;
export get;
export equal;
export clear;
export set_all;
export invert;
export set;
export is_true;
export is_false;
export to_vec;
export to_str;
export eq_vec;
export methods;

struct SmallBitv {
    let mut bits: u32;
    new(bits: u32) { self.bits = bits; }
    priv {
        #[inline(always)]
        fn bits_op(right_bits: u32, f: fn(u32, u32) -> u32) -> bool {
            let old_b: u32 = self.bits;
            let new_b = f(old_b, right_bits);
            self.bits = new_b;
            old_b != new_b
        }
    }
    #[inline(always)]
    fn union(s: &SmallBitv) -> bool {
        self.bits_op(s.bits, |u1, u2| { u1 | u2 })
    }
    #[inline(always)]
    fn intersect(s: &SmallBitv) -> bool {
        self.bits_op(s.bits, |u1, u2| { u1 & u2 })
    }
    #[inline(always)]
    fn become(s: &SmallBitv) -> bool {
        let old = self.bits;
        self.bits = s.bits;
        old != self.bits
    }
    #[inline(always)]
    fn difference(s: &SmallBitv) -> bool {
        let old = self.bits;
        self.bits &= !s.bits;
        old != self.bits
    }
    #[inline(always)]
    pure fn get(i: uint) -> bool {
        (self.bits & (1 << i)) != 0
    }
    #[inline(always)]
    fn set(i: uint, x: bool) {
        if x {
            self.bits |= 1<<i;
        }
        else {
            self.bits &= !(i as u32);
        }
    }
    #[inline(always)]
    fn equals(b: &SmallBitv) -> bool { self.bits == b.bits }
    #[inline(always)]
    fn clear() { self.bits = 0; }
    #[inline(always)]
    fn set_all() { self.bits = !0; }
    #[inline(always)]
    fn is_true() -> bool { self.bits == !0 }
    #[inline(always)]
    fn is_false() -> bool { self.bits == 0 }
    #[inline(always)]
    fn invert() { self.bits = !self.bits; }
}

struct BigBitv {
// only mut b/c of clone and lack of other constructor
    let mut storage: ~[mut uint];
    new(-storage: ~[mut uint]) {
        self.storage <- storage;
    }
    priv {
        #[inline(always)]
        fn process(b: &BigBitv, op: fn(uint, uint) -> uint) -> bool {
            let len = b.storage.len();
            assert (self.storage.len() == len);
            let mut changed = false;
            do uint::range(0, len) |i| {
                let w0 = self.storage[i];
                let w1 = b.storage[i];
                let w = op(w0, w1);
                if w0 != w unchecked { changed = true; self.storage[i] = w; };
                true
            };
            changed
        }
    }
    #[inline(always)]
     fn each_storage(op: fn(&uint) -> bool) {
        for uint::range(0, self.storage.len()) |i| {
            let mut w = self.storage[i];
            let b = !op(w);
            self.storage[i] = w;
            if !b { break; }
        }
     }
    #[inline(always)]
    fn invert() { for self.each_storage() |w| { w = !w } }
    #[inline(always)]
    fn union(b: &BigBitv)     -> bool { self.process(b, lor) }
    #[inline(always)]
    fn intersect(b: &BigBitv) -> bool { self.process(b, land) }
    #[inline(always)]
    fn become(b: &BigBitv)    -> bool { self.process(b, right) }
    #[inline(always)]
    fn difference(b: &BigBitv) -> bool {
        self.invert();
        let b = self.intersect(b);
        self.invert();
        b
    }
    #[inline(always)]
    pure fn get(i: uint) -> bool {
        let w = i / uint_bits;
        let b = i % uint_bits;
        let x = 1 & self.storage[w] >> b;
        x == 1
    }
    #[inline(always)]
    fn set(i: uint, x: bool) {
        let w = i / uint_bits;
        let b = i % uint_bits;
        let flag = 1 << b;
        self.storage[w] = if x { self.storage[w] | flag }
                 else { self.storage[w] & !flag };
    }
    #[inline(always)]
    fn equals(b: &BigBitv) -> bool {
        let len = b.storage.len();
        for uint::iterate(0, len) |i| {
            if self.storage[i] != b.storage[i] { return false; }
        }
    }
}

enum BitvVariant { Big(~BigBitv), Small(~SmallBitv) }

enum Op {Union, Intersect, Assign, Difference}

// The bitvector type
struct Bitv {
    let rep: BitvVariant;
    let nbits: uint;

    new(nbits: uint, init: bool) {
        self.nbits = nbits;
        if nbits <= 32 {
          self.rep = Small(~SmallBitv(if init {!0} else {0}));
        }
        else {
          let s = to_mut(from_elem(nbits / uint_bits + 1,
                                        if init {!0} else {0}));
          self.rep = Big(~BigBitv(s));
        };
    }

    priv {
        fn die() -> ! {
            fail ~"Tried to do operation on bit vectors with \
                  different sizes";
        }
        #[inline(always)]
        fn do_op(op: Op, other: &Bitv) -> bool {
            if self.nbits != other.nbits {
                self.die();
            }
            match self.rep {
              Small(s) => match other.rep {
                Small(s1) => match op {
                  Union      => s.union(s1),
                  Intersect  => s.intersect(s1),
                  Assign     => s.become(s1),
                  Difference => s.difference(s1)
                },
                Big(s1) => self.die()
              },
              Big(s) => match other.rep {
                Small(_) => self.die(),
                Big(s1) => match op {
                  Union      => s.union(s1),
                  Intersect  => s.intersect(s1),
                  Assign     => s.become(s1),
                  Difference => s.difference(s1)
                }
              }
            }
        }
    }

/**
 * Calculates the union of two bitvectors
 *
 * Sets `self` to the union of `self` and `v1`. Both bitvectors must be
 * the same length. Returns 'true' if `self` changed.
*/
    #[inline(always)]
    fn union(v1: &Bitv) -> bool { self.do_op(Union, v1) }

/**
 * Calculates the intersection of two bitvectors
 *
 * Sets `self` to the intersection of `self` and `v1`. Both bitvectors must be
 * the same length. Returns 'true' if `self` changed.
*/
    #[inline(always)]
    fn intersect(v1: &Bitv) -> bool { self.do_op(Intersect, v1) }

/**
 * Assigns the value of `v1` to `self`
 *
 * Both bitvectors must be the same length. Returns `true` if `self` was
 * changed
 */
    #[inline(always)]
    fn assign(v: &Bitv) -> bool { self.do_op(Assign, v) }

    /// Makes a copy of a bitvector
    #[inline(always)]
    fn clone() -> ~Bitv {
        ~match self.rep {
          Small(b) => {
            Bitv{nbits: self.nbits, rep: Small(~SmallBitv{bits: b.bits})}
          }
          Big(b) => {
            let st = to_mut(from_elem(self.nbits / uint_bits + 1, 0));
            let len = st.len();
            for uint::range(0, len) |i| { st[i] = b.storage[i]; };
            Bitv{nbits: self.nbits, rep: Big(~BigBitv{storage: st})}
          }
        }
    }

    /// Retrieve the value at index `i`
    #[inline(always)]
    pure fn get(i: uint) -> bool {
       assert (i < self.nbits);
       match self.rep {
         Big(b)   => b.get(i),
         Small(s) => s.get(i)
       }
    }

/**
 * Set the value of a bit at a given index
 *
 * `i` must be less than the length of the bitvector.
 */
    #[inline(always)]
    fn set(i: uint, x: bool) {
      assert (i < self.nbits);
      match self.rep {
        Big(b)   => b.set(i, x),
        Small(s) => s.set(i, x)
      }
    }

/**
 * Compares two bitvectors
 *
 * Both bitvectors must be the same length. Returns `true` if both bitvectors
 * contain identical elements.
 */
    #[inline(always)]
    fn equal(v1: Bitv) -> bool {
      if self.nbits != v1.nbits { return false; }
      match self.rep {
        Small(b) => match v1.rep {
          Small(b1) => b.equals(b1),
          _ => false
        },
        Big(s) => match v1.rep {
          Big(s1) => s.equals(s1),
          Small(_) => return false
        }
      }
    }

    /// Set all bits to 0
    #[inline(always)]
    fn clear() {
        match self.rep {
          Small(b) => b.clear(),
          Big(s) => for s.each_storage() |w| { w = 0u }
        }
    }

    /// Set all bits to 1
    #[inline(always)]
    fn set_all() {
      match self.rep {
        Small(b) => b.set_all(),
        Big(s) => for s.each_storage() |w| { w = !0u } }
    }

    /// Invert all bits
    #[inline(always)]
    fn invert() {
      match self.rep {
        Small(b) => b.invert(),
        Big(s) => for s.each_storage() |w| { w = !w } }
    }

/**
 * Calculate the difference between two bitvectors
 *
 * Sets each element of `v0` to the value of that element minus the element
 * of `v1` at the same index. Both bitvectors must be the same length.
 *
 * Returns `true` if `v0` was changed.
 */
   #[inline(always)]
    fn difference(v: ~Bitv) -> bool { self.do_op(Difference, v) }

        /// Returns true if all bits are 1
    #[inline(always)]
    fn is_true() -> bool {
      match self.rep {
        Small(b) => b.is_true(),
        _ => {
          for self.each() |i| { if !i { return false; } }
          true
        }
      }
    }

    #[inline(always)]
    fn each(f: fn(bool) -> bool) {
        let mut i = 0;
        while i < self.nbits {
            if !f(self.get(i)) { break; }
            i += 1;
        }
    }

    /// Returns true if all bits are 0

    fn is_false() -> bool {
      match self.rep {
        Small(b) => b.is_false(),
        Big(_) => {
          for self.each() |i| { if i { return false; } }
          true
        }
      }
    }

    fn init_to_vec(i: uint) -> uint {
      return if self.get(i) { 1 } else { 0 };
    }

/**
 * Converts `self` to a vector of uint with the same length.
 *
 * Each uint in the resulting vector has either value 0u or 1u.
 */
    fn to_vec() -> ~[uint] {
        vec::from_fn(self.nbits, |x| self.init_to_vec(x))
    }

/**
 * Converts `self` to a string.
 *
 * The resulting string has the same length as `self`, and each
 * character is either '0' or '1'.
 */
     fn to_str() -> ~str {
       let mut rs = ~"";
       for self.each() |i| { if i { rs += ~"1"; } else { rs += ~"0"; } };
       rs
     }


/**
 * Compare a bitvector to a vector of uint
 *
 * The uint vector is expected to only contain the values 0u and 1u. Both the
 * bitvector and vector must have the same length
 */
     fn eq_vec(v: ~[uint]) -> bool {
       assert self.nbits == v.len();
       let mut i = 0;
       while i < self.nbits {
           let w0 = self.get(i);
           let w1 = v[i];
           if !w0 && w1 != 0u || w0 && w1 == 0u { return false; }
           i = i + 1;
       }
       true
     }

    fn ones(f: fn(uint) -> bool) {
        for uint::range(0, self.nbits) |i| {
            if self.get(i) {
                if !f(i) { break }
            }
        }
    }

} // end of bitv class

const uint_bits: uint = 32u + (1u << 32u >> 27u);

pure fn lor(w0: uint, w1: uint) -> uint { return w0 | w1; }

pure fn land(w0: uint, w1: uint) -> uint { return w0 & w1; }

pure fn right(_w0: uint, w1: uint) -> uint { return w1; }

impl Bitv: ops::index<uint,bool> {
    pure fn index(&&i: uint) -> bool {
        self.get(i)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_to_str() {
        let zerolen = Bitv(0u, false);
        assert zerolen.to_str() == ~"";

        let eightbits = Bitv(8u, false);
        assert eightbits.to_str() == ~"00000000";
    }

    #[test]
    fn test_0_elements() {
        let mut act;
        let mut exp;
        act = Bitv(0u, false);
        exp = vec::from_elem::<uint>(0u, 0u);
        assert act.eq_vec(exp);
    }

    #[test]
    fn test_1_element() {
        let mut act;
        act = Bitv(1u, false);
        assert act.eq_vec(~[0u]);
        act = Bitv(1u, true);
        assert act.eq_vec(~[1u]);
    }

    #[test]
    fn test_10_elements() {
        let mut act;
        // all 0

        act = Bitv(10u, false);
        assert (act.eq_vec(~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u]));
        // all 1

        act = Bitv(10u, true);
        assert (act.eq_vec(~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv(10u, false);
        act.set(0u, true);
        act.set(1u, true);
        act.set(2u, true);
        act.set(3u, true);
        act.set(4u, true);
        assert (act.eq_vec(~[1u, 1u, 1u, 1u, 1u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv(10u, false);
        act.set(5u, true);
        act.set(6u, true);
        act.set(7u, true);
        act.set(8u, true);
        act.set(9u, true);
        assert (act.eq_vec(~[0u, 0u, 0u, 0u, 0u, 1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv(10u, false);
        act.set(0u, true);
        act.set(3u, true);
        act.set(6u, true);
        act.set(9u, true);
        assert (act.eq_vec(~[1u, 0u, 0u, 1u, 0u, 0u, 1u, 0u, 0u, 1u]));
    }

    #[test]
    fn test_31_elements() {
        let mut act;
        // all 0

        act = Bitv(31u, false);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u]));
        // all 1

        act = Bitv(31u, true);
        assert (act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv(31u, false);
        act.set(0u, true);
        act.set(1u, true);
        act.set(2u, true);
        act.set(3u, true);
        act.set(4u, true);
        act.set(5u, true);
        act.set(6u, true);
        act.set(7u, true);
        assert (act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv(31u, false);
        act.set(16u, true);
        act.set(17u, true);
        act.set(18u, true);
        act.set(19u, true);
        act.set(20u, true);
        act.set(21u, true);
        act.set(22u, true);
        act.set(23u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv(31u, false);
        act.set(24u, true);
        act.set(25u, true);
        act.set(26u, true);
        act.set(27u, true);
        act.set(28u, true);
        act.set(29u, true);
        act.set(30u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv(31u, false);
        act.set(3u, true);
        act.set(17u, true);
        act.set(30u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u]));
    }

    #[test]
    fn test_32_elements() {
        let mut act;
        // all 0

        act = Bitv(32u, false);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u]));
        // all 1

        act = Bitv(32u, true);
        assert (act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv(32u, false);
        act.set(0u, true);
        act.set(1u, true);
        act.set(2u, true);
        act.set(3u, true);
        act.set(4u, true);
        act.set(5u, true);
        act.set(6u, true);
        act.set(7u, true);
        assert (act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv(32u, false);
        act.set(16u, true);
        act.set(17u, true);
        act.set(18u, true);
        act.set(19u, true);
        act.set(20u, true);
        act.set(21u, true);
        act.set(22u, true);
        act.set(23u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv(32u, false);
        act.set(24u, true);
        act.set(25u, true);
        act.set(26u, true);
        act.set(27u, true);
        act.set(28u, true);
        act.set(29u, true);
        act.set(30u, true);
        act.set(31u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv(32u, false);
        act.set(3u, true);
        act.set(17u, true);
        act.set(30u, true);
        act.set(31u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 1u]));
    }

    #[test]
    fn test_33_elements() {
        let mut act;
        // all 0

        act = Bitv(33u, false);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u]));
        // all 1

        act = Bitv(33u, true);
        assert (act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv(33u, false);
        act.set(0u, true);
        act.set(1u, true);
        act.set(2u, true);
        act.set(3u, true);
        act.set(4u, true);
        act.set(5u, true);
        act.set(6u, true);
        act.set(7u, true);
        assert (act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv(33u, false);
        act.set(16u, true);
        act.set(17u, true);
        act.set(18u, true);
        act.set(19u, true);
        act.set(20u, true);
        act.set(21u, true);
        act.set(22u, true);
        act.set(23u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv(33u, false);
        act.set(24u, true);
        act.set(25u, true);
        act.set(26u, true);
        act.set(27u, true);
        act.set(28u, true);
        act.set(29u, true);
        act.set(30u, true);
        act.set(31u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 0u]));
        // mixed

        act = Bitv(33u, false);
        act.set(3u, true);
        act.set(17u, true);
        act.set(30u, true);
        act.set(31u, true);
        act.set(32u, true);
        assert (act.eq_vec(
                       ~[0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 1u, 1u]));
    }

    #[test]
    fn test_equal_differing_sizes() {
        let v0 = Bitv(10u, false);
        let v1 = Bitv(11u, false);
        assert !v0.equal(v1);
    }

    #[test]
    fn test_equal_greatly_differing_sizes() {
        let v0 = Bitv(10u, false);
        let v1 = Bitv(110u, false);
        assert !v0.equal(v1);
    }
}

//
// Local Variables:
// mode: rust
// fill-column: 78;
// indent-tabs-mode: nil
// c-basic-offset: 4
// buffer-file-coding-system: utf-8-unix
// End:
//
