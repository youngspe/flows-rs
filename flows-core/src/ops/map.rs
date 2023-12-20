use std::{
    ops::ControlFlow::{self, Break, Continue},
    pin::Pin,
    task::{Context, Poll},
};

use crate::{custom_fn::{MapFn, MapFnOnce}, my_try::MyTry, Flow};

use super::{FlowOp, WrapOp};

pin_project_lite::pin_project!(
    pub struct MapSync<Fl, Fun> {
        #[pin]
        flow: Fl,
        fun: Fun,
    }
);

impl<Fl, Fun, Res, Y1, Y2> Flow<Res> for MapSync<Fl, Fun>
where
    Fl: Flow<Res, Yield = Y1>,
    Fun: MapFn<Y1, Out = Y2>,
{
    type Yield = Y2;
    type Return = Fl::Return;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Res>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        let this = self.project();
        this.flow
            .poll_resume(cx, input)
            .map(|x| MyTry::map(x, |y| this.fun.map_exec(y)))
    }
    fn can_resume(self: Pin<&mut Self>) -> bool {
        self.project().flow.can_resume()
    }
}

pub fn map_sync<'f, Y, Res, Fl, Fun>(
    fun: Fun,
) -> WrapOp<impl FlowOp<Fl, Res, Output = MapSync<Fl, Fun>> + 'f>
where
    Fl: 'f + Flow<Res>,
    Fun: 'f + MapFn<Fl::Yield, Out = Y>,
{
    WrapOp(|flow: Fl| MapSync { flow, fun })
}

pin_project_lite::pin_project!(
    pub struct MapReturnSync<Fl, Fun> {
        #[pin]
        flow: Fl,
        fun: Option<Fun>,
    }
);

impl<Fl, Fun, Res, Ret1, Ret2> Flow<Res> for MapReturnSync<Fl, Fun>
where
    Fl: Flow<Res, Return = Ret1>,
    Fun: MapFnOnce<Ret1, Out = Ret2>,
{
    type Yield = Fl::Yield;
    type Return = Ret2;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Res>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        let this = self.project();
        this.flow.poll_resume(cx, input).map(|ctrl| match ctrl {
            Continue(yielded) => Continue(yielded),
            Break(returned) => Break(this.fun.take().unwrap().map_exec_once(returned)),
        })
    }
    fn can_resume(self: Pin<&mut Self>) -> bool {
        self.project().flow.can_resume()
    }
}

pub fn map_return_sync<'f, Fl, Ret, Res, Fun>(
    fun: Fun,
) -> WrapOp<impl FlowOp<Fl, Res, Output = MapReturnSync<Fl, Fun>> + 'f>
where
    Fl: 'f + Flow<Res>,
    Fun: 'f + MapFn<Fl::Return, Out = Ret>,
{
    WrapOp(|flow: Fl| MapReturnSync {
        flow,
        fun: Some(fun),
    })
}

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
