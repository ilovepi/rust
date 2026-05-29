mod foo;
fn main() {
    foo::greet();
    bar::hello();
    let x: () = "force error";
}
