#![deny(missing_docs)]

//! inline storage of `Any` type

use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    mem::transmute,
};

use typeless::TypeErased;

/// Like `Box<dyn Any>`, but stored inline (without allocations)
pub struct InlineAny<const C: usize> {
    data: TypeErased<C>,
    type_id: TypeId,
    drop_in_place: unsafe fn(*mut ()),
    __marker: PhantomData<dyn Any>,
}

impl<const C: usize> InlineAny<C> {
    /// Creates a new `InlineAny` containing the provided value
    pub fn new<T: Any>(value: T) -> Self {
        assert!(
            std::mem::size_of::<T>() <= C,
            "size of type too large for `InlineAny`"
        );
        assert!(
            std::mem::align_of::<T>() <= 8,
            "alignment of type too large for `InlineAny`"
        );
        let type_id = TypeId::of::<T>();
        // Safety: Still in question, see issue #2
        let drop_in_place = unsafe { transmute(std::ptr::drop_in_place::<T> as unsafe fn(*mut T)) };
        Self {
            // Safety: assertions above, and the `Any` bound requires `'static`
            data: unsafe { TypeErased::new_unchecked(value) },
            type_id,
            drop_in_place,
            __marker: PhantomData,
        }
    }

    /// Checks if the contained value is of the specified type
    #[inline]
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// Returns the `TypeId` of the contained type
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns some reference to the boxed value if it is of type T, or None if it isn’t.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        if self.is::<T>() {
            // Safety:
            // data is guaranteed to be of type `T`, there's no way to construct it where it isn't
            Some(unsafe { self.data.assume_type_ref() })
        } else {
            None
        }
    }

    /// Returns some mutable reference to the boxed value if it is of type T, or None if it isn’t.
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            // Safety:
            // data is guaranteed to be of type `T`, there's no way to construct it where it isn't
            Some(unsafe { self.data.assume_type_mut() })
        } else {
            None
        }
    }

    /// Attempt to downcast the box to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<T, Self> {
        if self.is::<T>() {
            // Safety:
            // data is guaranteed to be of type `T`,
            // there's no way to construct it where it isn't
            //
            // It's okay to have 2 copies of the data, since the raw bytes aren't interpreted as `T`
            // so it's really just this one copy of it.
            // As long as `self` is forgotten and `self.drop_in_place` isn't called, everything's fine
            let x = unsafe { self.data.as_ptr::<T>().read() };
            std::mem::forget(self);
            Ok(x)
        } else {
            Err(self)
        }
    }
}

impl<const C: usize> Drop for InlineAny<C> {
    fn drop(&mut self) {
        // Safety: Still in question, see issue #2
        unsafe { (self.drop_in_place)(self.data.as_mut_ptr::<()>()) }
    }
}
