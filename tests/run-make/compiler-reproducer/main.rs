#![feature(no_core)]
#![no_std]
#![no_core]

mod foo;
fn main() {
    foo::greet();
    bar::hello();
    let x: () = "force error";
}
