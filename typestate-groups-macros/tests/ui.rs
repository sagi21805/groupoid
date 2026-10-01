#[test]
#[cfg_attr(miri, ignore = "trybuild spawns cargo")]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
