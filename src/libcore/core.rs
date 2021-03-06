// Top-level, visible-everywhere definitions.

// Export various ubiquitous types, constructors, methods.

import option::{some, none};
import option = option::option;
import path = path::path;
import str::extensions;
import vec::extensions;
import option::extensions;
import option_iter::extensions;
import ptr::extensions;
import rand::extensions;
import result::extensions;
import int::num;
import i8::num;
import i16::num;
import i32::num;
import i64::num;
import uint::num;
import u8::num;
import u16::num;
import u32::num;
import u64::num;
import float::num;
import f32::num;
import f64::num;

export path, option, some, none, unreachable;
export extensions;
// The following exports are the extension impls for numeric types
export num;

// Export the log levels as global constants. Higher levels mean
// more-verbosity. Error is the bottom level, default logging level is
// warn-and-below.

export error, warn, info, debug;

#[doc = "The error log level"]
const error : u32 = 0_u32;
#[doc = "The warning log level"]
const warn : u32 = 1_u32;
#[doc = "The info log level"]
const info : u32 = 2_u32;
#[doc = "The debug log level"]
const debug : u32 = 3_u32;

// A curious inner-module that's not exported that contains the binding
// 'core' so that macro-expanded references to core::error and such
// can be resolved within libcore.
mod core {
    const error : u32 = 0_u32;
    const warn : u32 = 1_u32;
    const info : u32 = 2_u32;
    const debug : u32 = 3_u32;
}

// Similar to above. Some magic to make core testable.
#[cfg(test)]
mod std {
    use std(vers = "0.2");
    import std::test;
}

#[doc = "
A standard function to use to indicate unreachable code. Because the
function is guaranteed to fail typestate will correctly identify
any code paths following the appearance of this function as unreachable.
"]
fn unreachable() -> ! {
    fail "Internal error: entered unreachable code";
}

