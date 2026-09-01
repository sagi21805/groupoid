use crate::proto::machine::{A, MyStruct, StateA, StateB, StateC};

mod machine {
    pub struct StateA;
    pub struct StateB;
    pub struct StateC; // 1. Add your new state

    pub trait GroupMarker {
        type Meta;
    }

    pub struct GroupATag;
    impl GroupMarker for GroupATag {
        type Meta = usize;
    }

    pub struct GroupBTag;
    impl GroupMarker for GroupBTag {
        type Meta = u64;
    }

    pub trait Metadata {
        type Marker: GroupMarker;
    }

    impl Metadata for StateA {
        type Marker = GroupATag;
    }

    impl Metadata for StateB {
        type Marker = GroupBTag;
    }

    // 2. Assign StateC to GroupATag. It automatically gets Meta = usize
    // and shares StateA's trait implementations.
    impl Metadata for StateC {
        type Marker = GroupATag;
    }

    pub struct MyStruct<S: Metadata> {
        pub(crate) data: <S::Marker as GroupMarker>::Meta,
    }

    pub trait A {
        fn a(&self);
    }

    trait AHelper<Marker> {
        fn a(&self);
    }

    // 3. This single implementation covers both StateA and StateC
    impl<S: Metadata> AHelper<GroupATag> for MyStruct<S> {
        fn a(&self) {
            println!("Group A implementation!");
        }
    }

    impl<S: Metadata> AHelper<GroupBTag> for MyStruct<S> {
        fn a(&self) {
            println!("Group B implementation!");
        }
    }

    impl<S: Metadata> A for MyStruct<S>
    where
        Self: AHelper<S::Marker>,
    {
        fn a(&self) {
            <Self as AHelper<S::Marker>>::a(self)
        }
    }
}

#[test]
fn test() {
    let t_a: MyStruct<StateA> = MyStruct { data: 3_usize };
    t_a.a(); // Prints: Group A implementation!

    let t_b: MyStruct<StateB> = MyStruct { data: 3 };

    t_b.a();

    // 4. StateC works perfectly using the exact same logic
    let t_c: MyStruct<StateC> = MyStruct { data: 98_usize };
    t_c.a(); // Prints: Group A implementation!
}
