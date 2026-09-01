struct StateA;
struct StateB;
struct StateC;

#[groupoid::blueprint]
pub trait Metadata {
    type Meta;
    type Buffer;
    const GROUP_ID: u32;
    fn default_meta() -> Self::Meta;
}

#[include_states(StateA, StateC)]
pub struct GroupA;

impl Metadata for GroupA {
    type Meta = usize;
    type Buffer = [u8; 64];
    const GROUP_ID: u32 = 1;
    fn default_meta() -> Self::Meta {
        0
    }
}

#[include_states(StateB)]
pub struct GroupB;

impl Metadata for GroupB {
    type Meta = u64;
    type Buffer = [u8; 128];
    const GROUP_ID: u32 = 2;
    fn default_meta() -> Self::Meta {
        42
    }
}

// 3. The Target Types
pub struct MyStruct<S: Metadata> {
    data: S::Meta,
}
pub trait A {
    fn a(&self);
}

#[group_impl(GroupA)]
impl<S> A for MyStruct<S> {
    fn a(&self) {
        println!(
            "Group A impl. ID: {}, Default: {}",
            S::GROUP_ID,
            S::default_meta()
        );
    }
}

#[group_impl(GroupB)]
impl<S> A for MyStruct<S> {
    fn a(&self) {
        println!(
            "Group B impl. ID: {}, Default: {}",
            S::GROUP_ID,
            S::default_meta()
        );
    }
}
