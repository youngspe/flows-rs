use async_io::Timer;
use flows_util::Flow;
use std::time::Duration;

pub use flows_util::ops::*;

pub fn delay_each<'f, Res: 'f, Fl: 'f>(
    duration: Duration,
) -> WrapOp<
    impl FlowOp<Fl, Res, Output = impl Flow<Res, Yield = Fl::Yield, Return = Fl::Return> + 'f> + 'f,
>
where
    Fl: Flow<Res>,
{
    map_each!(
        #[capture(duration)]
        |item| {
            Timer::after(duration).await;
            item
        }
    )
}
