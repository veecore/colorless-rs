use core::marker::PhantomData;

#[inline(always)]
pub fn unify_value<T>(a: T, _b: PhantomData<T>) -> T {
    a
}
