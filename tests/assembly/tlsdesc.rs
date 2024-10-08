//@ revisions:x64 x64-trad x64-desc
//@ assembly-output: emit-asm
//@ aux-build:tlsdesc_aux.rs
//@ only-x86_64


//@[x64]      compile-flags: --target=x86_64-unknown-linux-gnu
//@[x64-trad] compile-flags: --target=x86_64-unknown-linux-gnu -Z tls-dialect=trad
//@[x64-desc] compile-flags: --target=x86_64-unknown-linux-gnu -Z tls-dialect=desc

#![crate_type = "lib"]

extern crate tlsdesc_aux as aux;

#[no_mangle]
fn get_aux() -> u64 {
    // x64:      __tls_get_addr
    // x64-trad: __tls_get_addr
    // x64-desc: tlsdesc
    // x64-desc: tlscall
    aux::A.with(|a| a.get())
}
