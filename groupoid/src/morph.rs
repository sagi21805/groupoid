/// Converts a wrapper around one type into the same wrapper around
/// another.
///
/// `morph_with` rebuilds tuples, ZSTs and bare projections itself, and
/// peels every other type one generic layer at a time through this trait.
/// `Option<[S::Value; 2]>` is morphed by `Option`'s impl, which calls
/// the array's impl, which calls `f`.
///
/// Only one generic argument of a layer may mention the state. The
/// built-in impls map the last one: the value of a map, or `Ok` of a
/// `Result`.
///
/// ```
/// use groupoid::{Morph, group, state, template, typestate};
///
/// #[template]
/// trait Meta {
///     type Value;
/// }
///
/// #[state]
/// struct Small;
/// #[state]
/// struct Big;
///
/// #[group(SmallGroup)]
/// impl Meta for (Small,) {
///     type Value = u8;
/// }
///
/// #[group(BigGroup)]
/// impl Meta for (Big,) {
///     type Value = u64;
/// }
///
/// struct Pair<T>(T, T);
///
/// impl<A, B> Morph<A, B> for Pair<A> {
///     type Output = Pair<B>;
///
///     fn morph(self, f: &mut impl FnMut(A) -> B) -> Pair<B> {
///         Pair(f(self.0), f(self.1))
///     }
/// }
///
/// #[typestate]
/// struct Wrap<S: Meta> {
///     pair: Pair<Pair<S::Value>>,
/// }
///
/// fn main() {
///     let small = Wrap::<Small> {
///         pair: Pair(Pair(1, 2), Pair(3, 4)),
///     };
///     let big: Wrap<Big> = small.morph_with(u64::from);
///     assert_eq!(big.pair.1.0, 3);
/// }
/// ```
///
/// # Features
///
/// The `core`, `alloc` and `std` features provide impls for the types of
/// those crates. With a feature off, the orphan rule still allows impls
/// for a local projection type, such as
/// `impl<B> Morph<MyValue, B> for Option<MyValue>`.
#[diagnostic::on_unimplemented(
    message = "implement `groupoid::Morph<{Src}, {Dst}>` for `{Self}` to \
               morph it",
    note = "for built-in types, enable the `core`, `alloc` or `std` \
            feature instead"
)]
pub trait Morph<Src, Dst> {
    /// `Self` with `Dst` in place of every `Src`.
    type Output;

    /// Rebuilds `self`, converting each `Src` with `f`.
    fn morph(self, f: &mut impl FnMut(Src) -> Dst) -> Self::Output;
}

#[cfg(feature = "core")]
mod core_impls {
    use core::{
        cell::{Cell, RefCell},
        cmp::Reverse,
        num::Wrapping,
    };

    use super::Morph;

    impl<A, B> Morph<A, B> for Option<A> {
        type Output = Option<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> Option<B> {
            self.map(f)
        }
    }

    impl<A, B, const N: usize> Morph<A, B> for [A; N] {
        type Output = [B; N];

        fn morph(self, f: &mut impl FnMut(A) -> B) -> [B; N] {
            self.map(f)
        }
    }

    /// Maps the `Ok` value.
    impl<A, B, E> Morph<A, B> for Result<A, E> {
        type Output = Result<B, E>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> Result<B, E> {
            self.map(f)
        }
    }

    impl<A, B> Morph<A, B> for Cell<A> {
        type Output = Cell<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> Cell<B> {
            Cell::new(f(self.into_inner()))
        }
    }

    impl<A, B> Morph<A, B> for RefCell<A> {
        type Output = RefCell<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> RefCell<B> {
            RefCell::new(f(self.into_inner()))
        }
    }

    impl<A, B> Morph<A, B> for Reverse<A> {
        type Output = Reverse<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> Reverse<B> {
            Reverse(f(self.0))
        }
    }

    impl<A, B> Morph<A, B> for Wrapping<A> {
        type Output = Wrapping<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> Wrapping<B> {
            Wrapping(f(self.0))
        }
    }
}

#[cfg(feature = "alloc")]
mod alloc_impls {
    use alloc::{
        boxed::Box,
        collections::{
            BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque,
        },
        vec::Vec,
    };

    use super::Morph;

    impl<A, B> Morph<A, B> for Box<A> {
        type Output = Box<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> Box<B> {
            Box::new(f(*self))
        }
    }

    impl<A, B> Morph<A, B> for Vec<A> {
        type Output = Vec<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> Vec<B> {
            self.into_iter().map(f).collect()
        }
    }

    impl<A, B> Morph<A, B> for VecDeque<A> {
        type Output = VecDeque<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> VecDeque<B> {
            self.into_iter().map(f).collect()
        }
    }

    impl<A, B> Morph<A, B> for LinkedList<A> {
        type Output = LinkedList<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> LinkedList<B> {
            self.into_iter().map(f).collect()
        }
    }

    impl<A, B: Ord> Morph<A, B> for BTreeSet<A> {
        type Output = BTreeSet<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> BTreeSet<B> {
            self.into_iter().map(f).collect()
        }
    }

    impl<A, B: Ord> Morph<A, B> for BinaryHeap<A> {
        type Output = BinaryHeap<B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> BinaryHeap<B> {
            self.into_iter().map(f).collect()
        }
    }

    /// Maps the values.
    impl<K: Ord, A, B> Morph<A, B> for BTreeMap<K, A> {
        type Output = BTreeMap<K, B>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> BTreeMap<K, B> {
            self.into_iter().map(|(k, v)| (k, f(v))).collect()
        }
    }
}

#[cfg(feature = "std")]
mod std_impls {
    use core::hash::{BuildHasher, Hash};
    use std::collections::{HashMap, HashSet};

    use super::Morph;

    impl<A, B: Eq + Hash, H: BuildHasher + Default> Morph<A, B>
        for HashSet<A, H>
    {
        type Output = HashSet<B, H>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> HashSet<B, H> {
            self.into_iter().map(f).collect()
        }
    }

    /// Maps the values.
    impl<K: Eq + Hash, A, B, H: BuildHasher + Default> Morph<A, B>
        for HashMap<K, A, H>
    {
        type Output = HashMap<K, B, H>;

        fn morph(self, f: &mut impl FnMut(A) -> B) -> HashMap<K, B, H> {
            self.into_iter().map(|(k, v)| (k, f(v))).collect()
        }
    }
}
