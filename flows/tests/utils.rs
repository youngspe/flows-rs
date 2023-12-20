use std::{pin::pin, time::Duration};

use async_io::Timer;
use futures::{select, Future, FutureExt};

pub async fn timeout<T>(ms: impl Into<Option<u64>>, block: impl Future<Output = T>) -> T {
    let ms = Option::unwrap_or(ms.into(), 10_000);
    let mut block = pin!(block.fuse());
    let mut timer = Timer::after(Duration::from_millis(ms)).fuse();

    select! {
        out = block => return out,
        _ = timer => panic!("timeout exceeded"),
    }
}

pub fn async_test(block: impl Future<Output = ()>) {
    futures::executor::block_on(timeout(None, block))
}
