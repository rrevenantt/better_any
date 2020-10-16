use better_typeid::Tid;
use better_typeid_derive::Tid;
struct S3<'a, T>(&'a T);
unsafe impl<'a, T> better_typeid::TidAble<'a> for S3<'a, T>
where
    T: better_typeid::TidAble<'a>,
{
    type Static = __S3aT_should_never_exist<T::Static>;
}
#[allow(warnings)]
#[doc(hidden)]
pub struct __S3aT_should_never_exist<T: ?Sized>(core::marker::PhantomData<T>);
struct S5<'a, T: Trait>(&'a T);
unsafe impl<'a, T: Trait> better_typeid::TidAble<'a> for S5<'a, T>
where
    T: better_typeid::TidAble<'a>,
{
    type Static = __S5aT_should_never_exist<T::Static>;
}
#[allow(warnings)]
#[doc(hidden)]
pub struct __S5aT_should_never_exist<T: ?Sized>(core::marker::PhantomData<T>);
struct S6<'a, T: TraitLT<'a>>(&'a T);
unsafe impl<'a, T: TraitLT<'a>> better_typeid::TidAble<'a> for S6<'a, T>
where
    T: better_typeid::TidAble<'a>,
{
    type Static = __S6aT_should_never_exist<T::Static>;
}
#[allow(warnings)]
#[doc(hidden)]
pub struct __S6aT_should_never_exist<T: ?Sized>(core::marker::PhantomData<T>);
struct S7<'a, T: 'static>(&'a T);
unsafe impl<'a, T: 'static> better_typeid::TidAble<'a> for S7<'a, T> {
    type Static = __S7aT_should_never_exist<T>;
}
#[allow(warnings)]
#[doc(hidden)]
pub struct __S7aT_should_never_exist<T: ?Sized>(core::marker::PhantomData<T>);
