import libc::{c_double, c_int};
import f64::*;

fn lgamma(n: c_double, value: &mut int) -> c_double {
  ret m::lgamma(n, value as &mut c_int);
}

#[link_name = "m"]
#[abi = "cdecl"]
native mod m {
    #[cfg(unix)]
    #[link_name="lgamma_r"] fn lgamma(n: c_double, sign: &mut c_int)
      -> c_double;
    #[cfg(windows)]
    #[link_name="__lgamma_r"] fn lgamma(n: c_double,
                                        sign: &mut c_int) -> c_double;

}

fn main() {
  let mut y: int = 5;
  let x: &mut int = &mut y;
  assert (lgamma(1.0 as c_double, x) == 0.0 as c_double);
}