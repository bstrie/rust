// error-pattern:explicit failure

use std;
import std::arc;

enum e<T: const send> { e(arc::Arc<T>) }

fn foo() -> e<int> {fail;}

fn main() {
   let f = foo();
}
