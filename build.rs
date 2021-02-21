fn main() {
    println!("cargo:rerun-if-changed=src/api/c/env.c");
    cc::Build::new().file("src/api/c/env.c").compile("env");
}
