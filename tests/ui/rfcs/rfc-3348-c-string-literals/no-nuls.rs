// edition: 2021

fn main() {
    c"\0";
    //~^ ERROR null characters in C string literals

    c"\u{00}";
    //~^ ERROR null characters in C string literals

    c" ";
    //~^ ERROR null characters in C string literals

    c"\x00";
    //~^ ERROR null characters in C string literals

    cr" ";
    //~^ ERROR null characters in C string literals
}

macro_rules! empty {
    ($($tt:tt)*) => {};
}

// The cfg consumes the literals before nul checking occurs.
#[cfg(FALSE)]
fn test() {
    c"\0";
    c"\u{00}";
    c" ";
    c"\x00";
    cr" ";
}

// The macro consumes the literals before nul checking occurs.
fn test_empty() {
    empty!(c"\0");
    empty!(c"\u{00}");
    empty!(c" ");
    empty!(c"\x00");
    empty!(cr" ");
}
