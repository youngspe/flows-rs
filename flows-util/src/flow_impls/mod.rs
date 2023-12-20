use std::{
    marker::PhantomData,
    ops::ControlFlow::{self, Continue},
    pin::Pin,
    task::{Context, Poll},
};

use flows_core::custom_fn::{CloneFn, MapFn, NewFn};

use super::Flow;

pub struct RepeatWith<F, Ret> {
    fun: F,
    _ret: PhantomData<Ret>,
}

impl<F, Res> Unpin for RepeatWith<F, Res> {}

impl<F, Ret> Flow for RepeatWith<F, Ret>
where
    F: NewFn,
{
    type Return = Ret;
    type Yield = F::Out;

    fn poll_resume(
        self: Pin<&mut Self>,
        _: &mut Context,
        input: &mut Option<()>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        if input.take().is_some() {
            Poll::Ready(Continue(self.get_mut().fun.new_exec()))
        } else {
            Poll::Pending
        }
    }
}

pub type Repeat<T, Ret> = RepeatWith<CloneFn<T>, Ret>;

pub fn repeat<T: Clone, Ret>(value: T) -> Repeat<T, Ret> {
    repeat_with(CloneFn(value))
}

pub fn repeat_with<F: NewFn, Ret>(fun: F) -> RepeatWith<F, Ret> {
    RepeatWith {
        fun,
        _ret: PhantomData,
    }
}

pub struct OnEachSync<F, Ret> {
    fun: F,
    _ret: PhantomData<Ret>,
}

impl<F, Ret> Unpin for OnEachSync<F, Ret> {}

impl<Y, Res, Ret, F> Flow<Res> for OnEachSync<F, Ret>
where
    F: MapFn<Res, Out = Y>,
{
    type Yield = Y;
    type Return = Ret;

    fn poll_resume(
        self: Pin<&mut Self>,
        _: &mut Context,
        input: &mut Option<Res>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        input
            .take()
            .map(|x| self.get_mut().fun.map_exec(x))
            .map(Continue)
            .map(Poll::Ready)
            .unwrap_or(Poll::Pending)
    }
}

pub fn on_each_sync<Res, Y, Ret, F>(fun: F) -> OnEachSync<F, Ret>
where
    F: MapFn<Res, Out = Y>,
{
    OnEachSync {
        fun,
        _ret: PhantomData,
    }
}

pub struct Identity<Ret> {
    _ret: PhantomData<Ret>,
}

impl<Res, Ret> Flow<Res> for Identity<Ret> {
    type Yield = Res;
    type Return = Ret;

    fn poll_resume(
        self: Pin<&mut Self>,
        _: &mut Context,
        input: &mut Option<Res>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        input
            .take()
            .map(Continue)
            .map(Poll::Ready)
            .unwrap_or(Poll::Pending)
    }
}

pub fn identity<Ret>() -> Identity<Ret> {
    Identity { _ret: PhantomData }
}
