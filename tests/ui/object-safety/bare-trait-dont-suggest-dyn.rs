// revisions: old new
//[old] edition:2015
//[new] edition:2021
//[new] run-rustfix
// FIXME: the test suite tries to create a crate called `bare_trait_dont_suggest_dyn.new`
#![crate_name="bare_trait_dont_suggest_dyn"]
#![deny(bare_trait_objects)]
fn ord_prefer_dot(s: String) -> Ord {
    //~^ ERROR the trait `Ord` cannot be made into an object
    //[old]~| ERROR trait objects without an explicit `dyn` are deprecated
    //[old]~| WARNING this is accepted in the current edition (Rust 2015)
    (s.starts_with("."), s)
}
fn main() {
    let _ = ord_prefer_dot(String::new());
}
