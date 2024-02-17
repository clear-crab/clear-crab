#![crate_type = "rlib"]
//@ no-prefer-dynamic

//@ compile-flags: -g

#[macro_use]
mod crate_with_invalid_spans_macros;

pub fn exported_generic<T>(x: T, y: u32) -> (T, u32) {
    // Using the add1 macro will produce an invalid span, because the `y` passed
    // to the macro will have a span from this file, but the rest of the code
    // generated from the macro will have spans from the macro-defining file.
    // The AST node for the (1 + y) expression generated by the macro will then
    // take it's `lo` span bound from the `1` literal in the macro-defining file
    // and it's `hi` bound from `y` in this file, which should be lower than the
    // `lo` and even lower than the lower bound of the SourceFile it is supposedly
    // contained in because the SourceFile for this file was allocated earlier than
    // the SourceFile of the macro-defining file.
    return (x, add1!(y));
}
