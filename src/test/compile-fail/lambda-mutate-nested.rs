// error-pattern:assigning to immutable variable declared in an outer block
// Make sure that nesting a block within a fn@ doesn't let us
// mutate upvars from a fn@.
fn f2(x: fn()) { x(); }

fn main() {
    let i = 0;
    let ctr = fn@ () -> int { f2({|| i = i + 1; }); ret i; };
    log(error, ctr());
    log(error, ctr());
    log(error, ctr());
    log(error, ctr());
    log(error, ctr());
    log(error, i);
}
