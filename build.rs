fn main() {
    cc::Build::new().file("src/c/env.c").compile("env");
}
