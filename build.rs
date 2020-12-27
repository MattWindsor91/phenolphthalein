fn main() {
    cc::Build::new().file("src/testapi/c/env.c").compile("env");
}
