fn main() {
    cc::Build::new().file("env.c").compile("env");

    cc::Build::new().file("test.c").compile("test");
}
