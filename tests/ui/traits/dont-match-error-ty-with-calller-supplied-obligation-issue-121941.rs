fn function<T: PartialEq>() {
    foo == 2; //~ ERROR cannot find value `foo` in this scope [E0425]
}

fn main() {}
