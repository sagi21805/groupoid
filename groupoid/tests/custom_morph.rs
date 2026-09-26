//! With the `core` feature off, a user implements `Morph` for `Option`
//! of a local projection type. Run with `--no-default-features`.
#![cfg(not(feature = "core"))]

use groupoid::{Morph, group, state, template, typestate};

#[derive(Debug, PartialEq)]
struct Meters(u32);

#[derive(Debug, PartialEq)]
struct Feet(u32);

impl<B> Morph<Meters, B> for Option<Meters> {
    type Output = Option<B>;

    fn morph(self, f: &mut impl FnMut(Meters) -> B) -> Option<B> {
        self.map(f)
    }
}

#[template]
trait Unit {
    type Value;
}

#[state]
struct Metric;
#[state]
struct Imperial;

#[group(MetricGroup)]
impl Unit for (Metric,) {
    type Value = Meters;
}

#[group(ImperialGroup)]
impl Unit for (Imperial,) {
    type Value = Feet;
}

#[typestate]
struct Length<S: Unit> {
    value: Option<S::Value>,
}

#[test]
fn user_impl_morphs_option() {
    let metric = Length::<Metric> {
        value: Some(Meters(3)),
    };
    let imperial: Length<Imperial> =
        metric.morph_with(|Meters(m)| Feet(m * 3));
    assert_eq!(imperial.value, Some(Feet(9)));
}
