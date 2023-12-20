mod compose;
mod for_each;
mod map;

use std::{
    ops::ControlFlow::{self, Break, Continue},
    pin::Pin,
    task::{Context, Poll},
};

use crate::{convert::IntoFlow, Flow};
pub use compose::*;
pub use for_each::*;
pub use map::*;

pub trait FlowOp<Fl: Flow<R>, R> {
    type Output;
    fn execute(self, flow: Fl) -> Self::Output;
}

impl<Fun: FnOnce(Fl) -> Out, Out, Fl: Flow<R>, R> FlowOp<Fl, R> for Fun {
    type Output = Out;
    fn execute(self, flow: Fl) -> Self::Output {
        self(flow)
    }
}

pub struct WrapOp<Op>(pub Op);

impl<Op> WrapOp<Op> {
    pub fn execute<Fl, R>(self, flow: Fl) -> Op::Output
    where
        Op: FlowOp<Fl, R>,
        Fl: Flow<R>,
    {
        self.0.execute(flow)
    }
}

impl<Op: FlowOp<Fl, R>, Fl: Flow<R>, R> FlowOp<Fl, R> for WrapOp<Op> {
    type Output = Op::Output;

    fn execute(self, flow: Fl) -> Self::Output {
        self.0.execute(flow)
    }
}

pin_project_lite::pin_project!(
    struct Latest<Fl: ?Sized> {
        #[pin]
        inner: Fl,
    }
);

impl<Fl, R> Flow<R> for Latest<Fl>
where
    Fl: ?Sized + Flow<R>,
    R: Default,
{
    type Yield = Fl::Yield;
    type Return = Fl::Return;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<R>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        let mut inner = self.project().inner;
        let mut alt_input = None;
        let mut out = Poll::Pending;

        loop {
            let input = &mut *input;
            let flow_input = if input.is_some() {
                input
            } else {
                if alt_input.is_none() && inner.as_mut().can_resume() {
                    alt_input = Some(Default::default());
                }
                &mut alt_input
            };

            match inner.as_mut().poll_resume(cx, flow_input) {
                cont @ Poll::Ready(Continue(..)) => {
                    out = cont;
                }
                ret @ Poll::Ready(Break(..)) => return ret,
                Poll::Pending => return out,
            }
        }
    }

    fn can_resume(self: Pin<&mut Self>) -> bool {
        self.project().inner.can_resume()
    }
}

pub fn latest<'f, Y, Res, Ret, Fl>(
) -> WrapOp<impl FlowOp<Fl, Res, Output = impl Flow<Res, Yield = Y, Return = Ret> + 'f> + 'f>
where
    Fl: 'f + Flow<Res, Yield = Y, Return = Ret>,
    Res: Default,
{
    WrapOp(|inner| Latest { inner })
}

pin_project_lite::pin_project!(
    struct Flatten<Fl1, Fl2, In, Ret> {
        #[pin]
        main_flow: Fl1,
        #[pin]
        sub_flow: Option<Fl2>,
        input: Option<In>,
        ret: Option<Ret>,
    }
);

impl<Fl1, Fl2, Yield, In, Res, Ret> Flow<Res> for Flatten<Fl1, Fl2, In, Ret>
where
    Fl1: Flow<In, Yield = Fl2, Return = Ret>,
    Fl2: Flow<Res, Yield = Yield, Return = In>,
    Res: Default,
{
    type Yield = Yield;
    type Return = Ret;

    fn poll_resume(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        input: &mut Option<Res>,
    ) -> Poll<ControlFlow<Self::Return, Self::Yield>> {
        loop {
            let mut this = self.as_mut().project();

            if this.ret.is_none() {
                match this.main_flow.poll_resume(cx, this.input) {
                    Poll::Ready(Continue(sub_flow)) => {
                        this.sub_flow.set(Some(sub_flow));
                    }
                    Poll::Ready(Break(output)) => {
                        *this.ret = Some(output);
                    }
                    Poll::Pending => {}
                }
            }

            let had_input = input.is_some();

            match this
                .sub_flow
                .as_mut()
                .as_pin_mut()
                .map(|fl| fl.poll_resume(cx, input))
            {
                Some(Poll::Ready(Continue(yielded))) => return Poll::Ready(Continue(yielded)),
                Some(Poll::Ready(Break(ret))) => {
                    this.sub_flow.set(None);
                    *this.input = Some(ret);
                    if had_input && input.is_none() {
                        *input = Some(Default::default());
                    }
                }
                Some(Poll::Pending) => return Poll::Pending,
                None => {
                    return this
                        .ret
                        .take()
                        .map(Break)
                        .map(Poll::Ready)
                        .unwrap_or(Poll::Pending)
                }
            }
        }
    }

    fn can_resume(self: Pin<&mut Self>) -> bool {
        self.project()
            .sub_flow
            .as_pin_mut()
            .map(Flow::can_resume)
            .unwrap_or(true)
    }
}

pub fn flatten<'f, Y, In, Res, Ret, M, Fl>(
) -> WrapOp<impl FlowOp<Fl, In, Output = impl Flow<Res, Yield = Y, Return = Ret> + 'f> + 'f>
where
    In: 'f + Default,
    Res: 'f + Default,
    Ret: 'f,
    M: 'f,
    Fl: 'f + Flow<In, Return = Ret>,
    Fl::Yield: IntoFlow<Res, M, Yield = Y, Return = In>,
{
    WrapOp(|main_flow: Fl| Flatten {
        main_flow: main_flow.then(map_sync(|fl: Fl::Yield| {
            fl.into_flow().then(map_return_sync(|_| Default::default()))
        })),
        sub_flow: None,
        input: Some(Default::default()),
        ret: None,
    })
}

pub fn flatten_init<'f, Y, In, Res, Ret, M, Fl>(
    init: In,
) -> WrapOp<impl FlowOp<Fl, In, Output = impl Flow<Res, Yield = Y, Return = Ret> + 'f> + 'f>
where
    In: 'f,
    Res: 'f + Default,
    Ret: 'f,
    M: 'f,
    Fl: 'f + Flow<In, Return = Ret>,
    Fl::Yield: IntoFlow<Res, M, Yield = Y, Return = In>,
{
    WrapOp(|main_flow: Fl| Flatten {
        main_flow: main_flow.then(map_sync(IntoFlow::into_flow)),
        sub_flow: None,
        input: Some(init),
        ret: None,
    })
}
