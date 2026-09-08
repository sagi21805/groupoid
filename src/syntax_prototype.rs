struct StateA;
struct StateB;
struct StateC;

#[blueprint]
pub trait Metadata {
    type Meta;
    type Buffer;
    const GROUP_ID: u32;
    fn default_meta() -> Self::Meta;
}

#[group(GroupA)]
impl Metadata for (StateA, StateB) {
    type Meta = u64;
    type Buffer = [u8; 128];
    const GROUP_ID: u32 = 2;
    fn default_meta() -> Self::Meta {
        42
    }
}

#[group(GroupB)]
impl Metadata for (StateA, StateB) {
    type Meta = usize;
    type Buffer = [u16; 128];
    const GROUP_ID: u32 = 3;
    fn default_meta() -> Self::Meta {
        42
    }
}

#[implements(A, B, state = S)]
pub struct MyStruct<S: Metadata> {
    data: S::Meta,
}

#[group_trait]
pub trait A {
    fn a(&self);
}

#[group_impl(GroupA)]
impl<S: Metadata> A for MyStruct<S> {
    fn a(&self) {
        println!(
            "Group A impl. ID: {}, Default: {}",
            S::GROUP_ID,
            S::default_meta()
        );
    }
}

#[group_impl(GroupB)]
impl<S: Metadata> A for MyStruct<S> {
    fn a(&self) {
        println!(
            "Group B impl. ID: {}, Default: {}",
            S::GROUP_ID,
            S::default_meta()
        );
    }
}
