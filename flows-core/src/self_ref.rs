use std::{
    marker::{PhantomData, PhantomPinned},
    pin::Pin,
};

pub type InvariantLt<'lt> = PhantomData<fn(&'lt ()) -> &'lt ()>;

pub trait WithLifetime<'lt> {
    type WithLt;
}

pub type Actual<'lt, W> = <W as WithLifetime<'lt>>::WithLt;

pub struct SelfRef<'x, T: ?Sized, R: ?Sized + for<'lt> WithLifetime<'lt>> {
    reference: Actual<'static, R>,
    _r: PhantomData<fn() -> (&'x R, &'x T)>,
    _pin: PhantomPinned,
    pub target: T,
}

impl<'x, T: ?Sized, R: ?Sized + for<'lt> WithLifetime<'lt>> Default for SelfRef<'x, T, R>
where
    T: Default,
    for<'lt> Actual<'lt, R>: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<'x, T: ?Sized, R: ?Sized + for<'lt> WithLifetime<'lt>> SelfRef<'x, T, R> {
    pub fn new(target: T) -> Self
    where
        T: Sized,
        for<'lt> Actual<'lt, R>: Default,
    {
        Self::new_with(target, |_| Default::default())
    }

    pub fn new_with(target: T, reference: impl FnOnce(InvariantLt) -> Actual<R>) -> Self
    where
        T: Sized,
    {
        Self {
            reference: reference(PhantomData),
            target,
            _r: PhantomData,
            _pin: PhantomPinned,
        }
    }

    pub fn reference(&self) -> &Actual<R> {
        unsafe { &*(&self.reference as *const Actual<'static, R>).cast() }
    }

    pub fn with_ref_mut<Out>(&mut self, f: impl FnOnce(&T, &mut Actual<R>) -> Out) -> Out {
        let target = &self.target;
        let reference = &mut self.reference;
        f(target, unsafe {
            &mut *((reference as *mut Actual<'static, R>).cast())
        })
    }

    pub fn with_mut<'this, Out>(
        self: Pin<&'this mut Self>,
        f: impl for<'lt> FnOnce(&'lt T, &'this mut Actual<'lt, R>) -> Out,
    ) -> Out
    where
        for<'lt> Actual<'lt, R>: Unpin,
    {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            let target = &this.target;
            let reference = &mut this.reference;
            f(
                target,
                &mut *((reference as *mut Actual<'static, R>).cast()),
            )
        }
    }

    pub fn with_pin_mut<'this, Out>(
        self: Pin<&'this mut Self>,
        f: impl for<'lt> FnOnce(&'lt T, Pin<&'this mut Actual<'lt, R>>) -> Out,
    ) -> Out {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            let target = &this.target;
            let reference = &mut this.reference;
            f(
                target,
                Pin::new_unchecked(&mut *((reference as *mut Actual<'static, R>).cast())),
            )
        }
    }
}
