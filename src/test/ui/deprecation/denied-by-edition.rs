// ignore-tidy-linelength

// TODO: test error on bad edition string

#![feature(staged_api)]

#![stable(feature = "denied-by-edition-test", since = "1.0.0")]

#[rustc_deprecated(since = "1.0.0", reason = "name too short", denied_by_edition="2015")]
#[stable(feature = "denied-by-edition-test", since = "1.0.0")]
pub struct S;

fn main() {
    let _ = S; //~ ERROR use of deprecated unit struct `S`: name too short
}
