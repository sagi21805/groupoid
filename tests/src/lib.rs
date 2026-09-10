use std::marker::PhantomData;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

struct Test<T> {
    _marker: PhantomData<T>,
}

impl<T: groupoid::State> groupoid::WithState for Test<T> {
    type State = T;
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
