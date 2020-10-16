//! # Better type Id
//!
//! Rust RFC for `non_static_type_id` feature has been reverted.
//! Which means in foreseeable future there will be no built-in way in rust to get type id for non-static type
//! let alone safely use it to downcast to a particular type.
//!
//! This crate provides tools to do these things safely for types with single lifetime.
//! Although looks like it is technically possible to extend this approach for multiple lifetimes,
//! consistent api and derive macro would be much harder to create and use because of the necessity
//! to properly handle lifetime relations.
//! Feel free to create an issue if you have actual use case where you need this functionality for multiple lifetimes.
//!
//! Also it has better downcasting that allows you do downcast not just from `dyn Tid` (like `dyn Any`) but from
//! any trait object that implements `Tid`.
//! So there is no more need to extend your traits with` fn to_any(&self)-> &dyn Any`
//!
//! ### Usage
//!
//! Basically in places where before you have used `dyn Any` you can use `dyn Tid<'a>`
//!  - If your type is generic you should derive `Tid` implementation for it with `Tid` derive macro.
//! Then to retrieve back concrete type `<dyn Tid>::downcast_*` methods should be used.
//!  - If your type is not generic/implements Any you can create `dyn Any` and convert it to `dyn Tid`.
//! Then to retrieve back concrete type `<dyn Tid>::downcast_any_*` methods should be used
//!  - If your type is not generic and local to your crate you also can derive `Tid` but then you need to be careful
//! to use methods that corresponds to the way you create `dyn Tid` for that particular type.
//! Otherwise downcasting will return `None`.
//!
//! If all your types can implement `Tid` to avoid confusion
//! recommended way is to use first option even if some types implement `Any`.
//! If there are some types that implement `Any` and can't implement `Tid` (i.e. types from other library),
//! recommended way is to use second option for all types that implement `Any` to reduce confusion to minimum.
//!
//! ### Interoperability with Any
//!
//! Unfortunately you can't just use `Tid` everywhere because currently it is impossible
//! to implement `Tid` for `T:Any` since it would conflict with any other possible `Tid` implementation.
//! To overcome this limitation there is a `From` impl to go from `dyn Any` to `dyn Tid`
//! But `Any` and `Tid` deliberately return different type ids because otherwise `Type<'a>` and `Type<'static>`
//! would be indistinguishable and it would allow to go from `Type<'static>` to `Type<'a>` via `dyn Tid`
//! which is obviously unsound for invariant and contravariant structs.
//!
//! Although if you are using `dyn Trait` where `Trait:Tid` all of this wouldn't work,
//! and you are left with `Tid` only.
//!
//! ### Limitations
//!
//! You can't get type id of unsized type. Supporting it would make api much more confusing, while
//! it can be worked around just by wrapping with another type.
//!
//! ### Safety
//!
//! It is safe because created trait object preserve lifetime information,
//! thus allowing us to safely downcast with proper lifetime.
//! Otherwise internally it is plain old `Any`.
use std::any::{Any, TypeId};
use std::marker::PhantomData;

/// Attribute macro that makes your implementation of `TidAble` safe
/// Use it when you can't use derive e.g. for trait object.
///
/// ```rust
/// # use better_typeid::{TidAble,impl_tid};
/// trait Trait<'a>{}
/// #[impl_tid]
/// impl<'a> TidAble<'a> for Box<dyn Trait<'a> + 'a>{}
/// ```
pub use better_typeid_derive::impl_tid;
/// Derive macro to implement traits from this crate
///
/// It checks if it is safe to implement `Tid` for your struct
/// Also it adds `:TidAble<'a>` bound on type parameters
/// unless your type parameter already has **explicit** `'static`
pub use better_typeid_derive::Tid;

/// This trait indicates that you can substitute this type as a type parameter to
/// another type so that resulting type could implement `Tid`.
///
/// So if you don't have such generic types, just use `Tid` everywhere,
/// you don't need to use this trait at all.
///
/// Only this trait is actually being implemented on user side.
/// Other traits are mostly just blanket implementations over X:TidAble<'a>
///
/// Note that this trait interfere with object safety, so you shouldn't use it as a super trait
/// if you are going to make a trait object.
/// Technically it is object safe, but you can't make a trait object from it without specifying internal associate type
/// like: `dyn TidAble<'a,Static=SomeType>` which make this trait object effectively useless.
///
/// Unsafe because safety of this crate relies on correctness of this trait implementation.
/// There are several safe ways to implement it:
///  - `#[derive(Tid)]` derive macro
///  - `type_id` declarative macro
///  - `impl_tid` attribute macro
// we need to have associate type because it allows TypeIdAdjuster to be a private type
// and allows to implement it for other trait objects
pub unsafe trait TidAble<'a>: Tid<'a> {
    /// Implementation detail
    #[doc(hidden)]
    type Static: ?Sized + Any;
}

/// Contains extension methods for actual downcasting.
///
/// Use methods from this trait only if `dyn Tid` was created directly from `T` for this particular `T`
pub trait TidExt<'a> {
    /// Use it only if `dyn Tid` was created directly from `T` for this particular `T`
    fn downcast_ref<'b, T: Tid<'a>>(&'b self) -> Option<&'b T>
    where
        'a: 'b;

    fn downcast_mut<'b, T: Tid<'a>>(&'b mut self) -> Option<&'b mut T>
    where
        'a: 'b;

    fn downcast_rc<T: Tid<'a>>(self: Rc<Self>) -> Result<Rc<T>, Rc<Self>>;

    fn downcast_arc<T: Tid<'a>>(self: Arc<Self>) -> Result<Arc<T>, Arc<Self>>;

    fn downcast_box<T: Tid<'a>>(self: Box<Self>) -> Result<Box<T>, Box<Self>>;
}

/// If X is Sized then any of those calls is optimized to no-op because both T and Self are known statically.
/// Useful if you have generic code that you want to behave differently depending on which
/// concrete type replaces type parameter. Usually there are better ways to do this like specialization,
/// but sometimes it can be the only way.
impl<'a, X: ?Sized + Tid<'a>> TidExt<'a> for X {
    #[inline]
    fn downcast_ref<'b, T: Tid<'a>>(&'b self) -> Option<&'b T>
    where
        'a: 'b,
    {
        // Tid<'a> is implemented only for types with lifetime 'a
        // so we can safely cast type back because lifetime invariant is preserved.
        if self.self_id() == T::id() {
            Some(unsafe { &*(self as *const _ as *const T) })
        } else {
            None
        }
    }

    #[inline]
    fn downcast_mut<'b, T: Tid<'a>>(&'b mut self) -> Option<&'b mut T>
    where
        'a: 'b,
    {
        // see downcast_ref
        if self.self_id() == T::id() {
            Some(unsafe { &mut *(self as *mut _ as *mut T) })
        } else {
            None
        }
    }

    #[inline]
    fn downcast_rc<T: Tid<'a>>(self: Rc<Self>) -> Result<Rc<T>, Rc<Self>> {
        if T::id() == self.self_id() {
            unsafe { Ok(Rc::from_raw(Rc::into_raw(self) as *const _)) }
        } else {
            Err(self)
        }
    }
    #[inline]
    fn downcast_arc<T: Tid<'a>>(self: Arc<Self>) -> Result<Arc<T>, Arc<Self>> {
        if T::id() == self.self_id() {
            unsafe { Ok(Arc::from_raw(Arc::into_raw(self) as *const _)) }
        } else {
            Err(self)
        }
    }

    #[inline]
    fn downcast_box<T: Tid<'a>>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        if T::id() == self.self_id() {
            unsafe { Ok(Box::from_raw(Box::into_raw(self) as *mut _)) }
        } else {
            Err(self)
        }
    }
}

/// This trait indicates that this type can be converted to
/// trait object with typeid while preserving lifetime information.
/// Extends `Any` functionality for types with single lifetime
///
/// Use it only as a `dyn Tid` or as super trait when you need to create trait object.
/// In all other places use `TidAble<'a>`.
///
/// Lifetime here is necessary to make `dyn Tid<'a> + 'a` invariant over `'a`.
pub unsafe trait Tid<'a>: 'a {
    /// Returns type id of the type of `self`
    ///
    /// Note that returned type id is guaranteed to be different from provided by `Any`.
    /// It is necessary for the creation of `dyn Tid` from `dyn Any` to be sound.
    fn self_id(&self) -> TypeId;

    /// Returns type id of this type
    fn id() -> TypeId
    where
        Self: Sized;
}

unsafe impl<'a, T: ?Sized + TidAble<'a>> Tid<'a> for T {
    #[inline]
    fn self_id(&self) -> TypeId {
        adjust_id::<T::Static>()
    }

    fn id() -> TypeId
    where
        Self: Sized,
    {
        adjust_id::<T::Static>()
    }
}

// this exists just to make TypeIdAdjuster private so type id difference between
// `dyn Any` and `dyn Tid` would be guaranteed
fn adjust_id<T: ?Sized + Any>() -> TypeId {
    TypeId::of::<TypeIdAdjuster<T>>()
}

/// Returns type id of `T`
///
/// Use it only if `Tid::id()` is not enough when `T` is not sized
pub fn typeid_of<'a, T: ?Sized + TidAble<'a>>() -> TypeId {
    adjust_id::<T::Static>()
}

impl<'a> From<Box<dyn Any>> for Box<dyn Tid<'a> + 'a> {
    #[inline]
    fn from(f: Box<dyn Any>) -> Self {
        // it should be safe because both Any and Tid have single entry in vtable
        // so there is no need to rely on the function order stability of the different traits
        // also, despite particular trait object layout is not stable, it still should be
        // the same for different trait objects in the same compilation pass.
        //todo find out more:
        // technically i think vtable also has a drop entry so different order still can happen
        // in theory but practically it is either always first or always last so it
        // shouldn't influence order
        unsafe { core::mem::transmute(f) }
    }
}

impl<'a: 'b, 'b> From<&'b dyn Any> for &'b (dyn Tid<'a> + 'a) {
    #[inline]
    fn from(f: &'b dyn Any) -> Self {
        unsafe { core::mem::transmute(f) }
    }
}

impl<'a: 'b, 'b> From<&'b mut dyn Any> for &'b mut (dyn Tid<'a> + 'a) {
    #[inline]
    fn from(f: &'b mut dyn Any) -> Self {
        unsafe { core::mem::transmute(f) }
    }
}

// this reverse is not possible even for 'static
// because otherwise drop could have been called after the end of lifetime
// and even for 'static it is not possible because it would allow to go from
// `dyn Tid<'a> + 'static` to `dyn Tid<'a> + 'a` which is effectively shortens invariant lifetime
// and it is a problem if initial struct is contravariant over lifetime
// impl From<Box<dyn Any>> for Box<dyn Tid<'a> + 'static> {
//     fn from(f: Box<dyn Any>) -> Self {
//         unsafe { core::mem::transmute(f) }
//     }
// }

//wrapper to distinguish type ids coming from `dyn Any` and `dyn Tid`
struct TypeIdAdjuster<T: ?Sized>(PhantomData<T>);

// todo check if we can relax it to `dyn Tid<'a> + 'b where 'b:'a`
// at first glance it should be possible because
// there should be a reason why trait object is allowed to be covariant over its lifetime
// and even if it is possible any use case i can imagine feels very artificial
// on the other hand `dyn Tid<'a> + 'b where 'b:'a`
// can be subtyped to `dyn Tid<'a> + 'a` on the caller side so this shouldn't even be a problem
// impl<'a> TidExt<'a> for dyn Tid<'a> + 'a {
//     #[inline]
//     fn downcast_ref<'b, T: Tid<'a>>(&'b self) -> Option<&'b T>
//     where
//         'a: 'b,
//     {
//         // Tid<'a> is implemented only for types with lifetime 'a
//         // so we can safely cast type back because lifetime invariant is preserved.
//         if self.self_id() == typeid_of::<T>() {
//             Some(unsafe { &*(self as *const _ as *const T) })
//         } else {
//             None
//         }
//     }
//
//     #[inline]
//     fn downcast_mut<'b, T: Tid<'a>>(&'b mut self) -> Option<&'b mut T>
//     where
//         'a: 'b,
//     {
//         // see downcast_ref
//         if self.self_id() == typeid_of::<T>() {
//             Some(unsafe { &mut *(self as *mut _ as *mut T) })
//         } else {
//             None
//         }
//     }
// }

impl<'a> dyn Tid<'a> + 'a {
    /// Tries to downcast Self to `T`
    ///
    /// Use it only if `dyn Tid` was created from `dyn Any` for this particular `T`
    ///
    /// ```rust
    /// # use std::any::Any;
    /// # use better_typeid::{Tid, TidExt};
    /// #[derive(Tid)]
    /// struct S;
    ///
    /// let a = &S as &dyn Any;
    /// let from_any: &dyn Tid = a.into();
    /// assert!(from_any.downcast_any_ref::<S>().is_some());
    /// assert!(from_any.downcast_ref::<S>().is_none());
    ///
    /// let direct = &S as &dyn Tid;
    /// assert!(direct.downcast_any_ref::<S>().is_none());
    /// assert!(direct.downcast_ref::<S>().is_some());
    /// ```
    #[inline]
    pub fn downcast_any_ref<T: Any>(&self) -> Option<&T> {
        // this condition can be true if and only if dyn Tid was created from dyn Any
        // because otherwise TypeId would be from TypeIdAdjuster<T> which cant be equal to the one of T
        // thus it is safe to increase lifetime back to 'static
        if self.self_id() == TypeId::of::<T>() {
            Some(unsafe { &*(self as *const _ as *const T) })
        } else {
            None
        }
    }

    /// Use it only if `dyn Tid` was created from `dyn Any` for this particular `T`
    #[inline]
    pub fn downcast_any_mut<T: Any>(&mut self) -> Option<&mut T> {
        // see downcast_any_ref
        if self.self_id() == TypeId::of::<T>() {
            Some(unsafe { &mut *(self as *mut _ as *mut T) })
        } else {
            None
        }
    }
}

macro_rules! stdimpl {
    ($struct: tt) => {
        unsafe impl<'a, T: ?Sized + TidAble<'a>> TidAble<'a> for $struct<T> {
            type Static = $struct<T::Static>;
        }
    };
}
use std::cell::*;
use std::ops::Deref;
use std::rc::*;
use std::sync::*;
stdimpl!(Box);
stdimpl!(Rc);
stdimpl!(RefCell);
stdimpl!(Cell);
stdimpl!(Arc);
stdimpl!(Mutex);
stdimpl!(RwLock);

#[impl_tid]
impl<'a, T> TidAble<'a> for Option<T> {}

#[impl_tid]
impl<'a, T> TidAble<'a> for Vec<T> {}

#[impl_tid]
impl<'a, T, E> TidAble<'a> for Result<T, E> {}

#[impl_tid]
impl<'a> TidAble<'a> for dyn Tid<'a> + 'a {}

// the logic behind this implementations is to connect Any with Tid somehow
// I would say that if T:Any there is no much need to implement Tid<'a> for T.
// because Any functionality already exists and `dyn Any` can be converted to `dyn Tid`.
// unfortunately there is no way to implement Tid<'a> for T:Any,
// which make impl<'a, T: Tid<'a>> Tid<'a> for &'a T {} almost useless
// because it wouldn't work even for &'a i32
// This way we don't require user to newtype wrapping simple references.
// And more complex types are usually not used as a type parameters directly.

unsafe impl<'a, T: Any> TidAble<'a> for &'a T {
    type Static = &'static T;
}

unsafe impl<'a, T: Any> TidAble<'a> for &'a mut T {
    type Static = &'static mut T;
}

// whole impl can be via this macro but it would require to use `paste` crate
// which is already a proc macro, so there is no much reason to do force everything to declarative macro
//
/// Simple version of derive macro to not pull all proc macro dependencies in simple cases
#[macro_export]
macro_rules! type_id {
    ($struct: ident) => {
        unsafe impl<'a> $crate::TidAble<'a> for $struct {
            type Static = $struct;
        }
    };
    ($struct: ident < $lt: lifetime >) => {
        unsafe impl<'a> $crate::TidAble<'a> for $struct<'a> {
            type Static = $struct<'static>;
        }
    }; // ($struct: ident < $($type:ident),* >) => {
       //     unsafe impl<'a,$($type:$crate::TidAble<'a>),*> $crate::TidAble<'a> for $struct<$($type,)*> {
       //         type Static = HygyieneUnique<$($type::Static),*>;
       //     }
       //
       //     pub struct HygyieneUnique<$($type),*>($(core::marker::PhantomData<$type>),*);
       // };
}

//todo: not sure if worth it.
// If only both T and X could be ?Sized it would DRY From impl as well
//
// unsafe trait Cast<T: Deref>: Deref {
//     unsafe fn cast(self) -> T;
// }
//
// unsafe impl<T: ?Sized, X> Cast<Rc<X>> for Rc<T> {
//     unsafe fn cast(self) -> Rc<X> {
//         Rc::from_raw(Rc::into_raw(self) as *const _)
//     }
// }
//
// unsafe impl<T: ?Sized, X> Cast<Arc<X>> for Arc<T> {
//     unsafe fn cast(self) -> Arc<X> {
//         Arc::from_raw(Arc::into_raw(self) as *const _)
//     }
// }
//
// unsafe impl<T: ?Sized, X> Cast<Box<X>> for Box<T> {
//     unsafe fn cast(self) -> Box<X> {
//         Box::from_raw(Box::into_raw(self) as *mut _)
//     }
// }
//
// // impl<'a, F: Cast<T, Target = dyn Any>, T: Deref<Target = dyn Tid<'a> + 'a>> From<F> for T {
// //     fn from(f: F) -> Self {
// //         unsafe { f.cast() }
// //     }
// // }
//
// fn downcast<'a, F: Cast<T>, T:Deref>(f: F) -> Result<T, F>
// where
//     F::Target: Tid<'a>,
//     T::Target: Tid<'a>,
// {
//     if <F::Target>::id() == f.self_id() {
//         unsafe { Ok(f.cast()) }
//     } else {
//         Err(f)
//     }
// }
