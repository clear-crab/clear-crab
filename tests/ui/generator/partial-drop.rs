// check-pass
#![feature(negative_impls, generators)]

struct Foo;
impl !Send for Foo {}

struct Bar {
    foo: Foo,
    x: i32,
}

fn main() {
    assert_send(|| {
        let guard = Bar { foo: Foo, x: 42 };
        drop(guard.foo);
        yield;
    });

    assert_send(|| {
        let mut guard = Bar { foo: Foo, x: 42 };
        drop(guard);
        guard = Bar { foo: Foo, x: 23 };
        yield;
    });

    assert_send(|| {
        let guard = Bar { foo: Foo, x: 42 };
        let Bar { foo, x } = guard;
        drop(foo);
        yield;
    });
}

fn assert_send<T: Send>(_: T) {}
