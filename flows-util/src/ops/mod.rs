use core::task;
use std::{
    marker::PhantomData,
    ops::ControlFlow::{self, Break, Continue},
    pin::Pin,
    task::{Context, Poll},
};

pub use flows_core::ops::*;
use flows_core::{convert::IntoFlow, custom_fn::MapFn, my_try::MyTry, Flow};

pub use flows_macros::{
    concat_map, filter, for_each, map_each, map_return, merge_map, switch_map, transform_each,
    try_for_each, try_transform_each,
};

use crate::flow_impls::{on_each_sync, OnEachSync};

pin_project_lite::pin_project!(
    pub struct Zip<F0, F1, R0, R1, Y0, Y1> {
        #[pin]
        f0: F0,
        #[pin]
        f1: F1,
        res: (Option<R0>, Option<R1>),
        yld: (Option<Y0>, Option<Y1>),
    }
);

impl<R0, R1, Y0, Y1, Ret, F0, F1> Flow<(R0, R1)> for Zip<F0, F1, R0, R1, Y0, Y1>
where
    F0: Flow<R0, Yield = Y0, Return = Ret>,
    F1: Flow<R1, Yield = Y1, Return = Ret>,
{
    type Yield = (Y0, Y1);
    type Return = Ret;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<(R0, R1)>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        let mut this = self.project();

        let mut inputs = match input.take() {
            Some((v1, v2)) => {
                *this.res = Default::default();
                (Some(v1), Some(v2))
            }
            None => (None, None),
        };

        fn inner<Res, F>(
            f: Pin<&mut F>,
            input: &mut Option<Res>,
            res: &mut Option<Res>,
            yld: &mut Option<F::Yield>,
            cx: &mut Context,
        ) -> Poll<ControlFlow<F::Return>>
        where
            F: Flow<Res>,
        {
            f.poll_resume(cx, if input.is_some() { input } else { res })
                .map(|ctrl| {
                    MyTry::map(ctrl, |value| {
                        *yld = Some(value);
                    })
                })
        }

        let polls = (
            inner(
                this.f0.as_mut(),
                &mut inputs.0,
                &mut this.res.0,
                &mut this.yld.0,
                cx,
            ),
            inner(
                this.f1.as_mut(),
                &mut inputs.1,
                &mut this.res.1,
                &mut this.yld.1,
                cx,
            ),
        );

        match inputs {
            (Some(r0), Some(r1)) => {
                *input = Some((r0, r1));
            }
            inputs => {
                *this.res = inputs;
            }
        }

        match polls {
            (Poll::Ready(Break(ret)), _) | (_, Poll::Ready(Break(ret))) => {
                return Poll::Ready(Break(ret))
            }
            (Poll::Pending, Poll::Pending) => return Poll::Pending,
            _ => {
                if let (Some(_), Some(_)) = this.yld {
                    Poll::Ready(Continue((
                        this.yld.0.take().unwrap(),
                        this.yld.1.take().unwrap(),
                    )))
                } else {
                    Poll::Pending
                }
            }
        }
    }

    fn can_resume(self: Pin<&mut Self>) -> bool {
        let this = self.project();
        this.f0.can_resume() || this.f1.can_resume()
    }
}

pub fn zip_with<Res0, Res1, Fl0, Fl1, M>(
    rhs: impl IntoFlow<Res1, M, IntoFlow = Fl1>,
) -> WrapOp<impl FlowOp<Fl0, Res0, Output = Zip<Fl0, Fl1, Res0, Res1, Fl0::Yield, Fl1::Yield>>>
where
    Fl0: Flow<Res0>,
    Fl1: Flow<Res1>,
{
    WrapOp(|lhs| zip(lhs, rhs))
}

pub fn zip<Res0, Res1, Fl0, Fl1, M0, M1>(
    f0: impl IntoFlow<Res0, M0, IntoFlow = Fl0>,
    f1: impl IntoFlow<Res1, M1, IntoFlow = Fl1>,
) -> Zip<Fl0, Fl1, Res0, Res1, Fl0::Yield, Fl1::Yield>
where
    Fl0: Flow<Res0>,
    Fl1: Flow<Res1>,
{
    Zip {
        f0: f0.into_flow(),
        f1: f1.into_flow(),
        res: Default::default(),
        yld: Default::default(),
    }
}

pin_project_lite::pin_project!(
    pub struct DiscardInput<Fl, Res> {
        #[pin]
        inner: Fl,
        _res: PhantomData<Res>,
    }
);

impl<Res1, Res2, Fl> Flow<Res2> for DiscardInput<Fl, Res1>
where
    Fl: Flow<Res1>,
    Res1: Default,
{
    type Yield = Fl::Yield;
    type Return = Fl::Return;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Res2>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        let mut default_input = if input.is_some() {
            Some(Res1::default())
        } else {
            None
        };

        let poll = self.project().inner.poll_resume(cx, &mut default_input);
        if default_input.is_none() {
            *input = None;
        }
        poll
    }

    fn can_resume(self: Pin<&mut Self>) -> bool {
        self.project().inner.can_resume()
    }
}

pub fn discard_input<Res1, Res2, Fl>(
) -> WrapOp<impl FlowOp<Fl, Res1, Output = DiscardInput<Fl, Res2>>>
where
    Fl: Flow<Res1>,
{
    WrapOp(|inner| DiscardInput {
        _res: PhantomData,
        inner,
    })
}

pub fn map_input<'f, A, B, F, Fl>(
    fun: F,
) -> WrapOp<impl FlowOp<Fl, B, Output = Compose<B, OnEachSync<F, Fl::Return>, Fl>> + 'f>
where
    F: 'f + MapFn<A, Out = B>,
    Fl: 'f + Flow<B>,
{
    WrapOp(|src: Fl| on_each_sync(fun).then(compose_with(src)))
}

pin_project_lite::pin_project!(
    pub struct TryUnwrap<Fl> {
        #[pin]
        inner: Fl,
    }
);

impl<Res, Y, Ret, Fl> Flow<Res> for TryUnwrap<Fl>
where
    Fl: Flow<Res, Yield = Y, Return = Ret>,
    Y: MyTry,
{
    type Yield = Y::Continue;
    type Return = Y::Mapped<Ret>;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Res>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        let this = self.project();
        Poll::Ready(match task::ready!(this.inner.poll_resume(cx, input)) {
            Continue(out) => match out.into_control_flow() {
                Continue(yld) => Continue(yld),
                Break(brk) => Break(MyTry::from_break(brk)),
            },
            Break(ret) => Break(MyTry::from_continue(ret)),
        })
    }
}

pub fn try_unwrap<'f, Res, Fl: 'f>() -> WrapOp<impl FlowOp<Fl, Res, Output = TryUnwrap<Fl>> + 'f>
where
    Fl: Flow<Res>,
{
    WrapOp(|inner| TryUnwrap { inner })
}
