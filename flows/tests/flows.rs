mod utils;

use std::{convert::identity, time::Duration};

use either::Either;
use flows::{
    flow, flow_of,
    ops::{
        concat_map, delay_each, filter, flatten, for_each, transform_each, try_for_each,
        try_transform_each, zip,
    },
    Flow, FromFlow, IntoFlow,
};

use crate::utils::async_test;

async fn sleep(ms: u64) {
    async_io::Timer::after(std::time::Duration::from_millis(ms)).await;
}

#[test]
fn try_for_each_break() {
    async_test(async {
        let f = flow!(|| {
            let mut i = 0i32;
            loop {
                next!(i);
                i += 1;
            }
        });

        let mut out = Vec::new();

        let ret = {
            let out = &mut out;
            f.then(try_for_each!(move |x| {
                out.push(x);

                if x == 10 {
                    Err(123)
                } else {
                    Ok(())
                }
            }))
            .await
        };

        let ret = ret.unwrap_or_else(identity);

        assert_eq!(out, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(ret, 123);
    })
}

#[test]
fn flow_with_next_from() {
    async_test(async {
        fn check_send<T: Send>(x: T) -> T {
            x
        }

        let f1 = check_send(flow!(|| {
            for i in 1..=5 {
                next!(i);
            }
            "foo"
        }));

        let f2 = check_send(flow!(|_| {
            next!(99);
            let ret = next_from!(flow!(|_| {
                f1.then(for_each!(move |x| {
                    next!(x * 2);
                    sleep(10).await;
                    next!(x * 2 + 1);
                }))
                .await
            }));
            next!(100);
            ret
        }));

        let mut out = Vec::new();

        let ret = f2
            .then(for_each!(
                #[capture(ref mut out)]
                |x| {
                    out.push(x);
                },
            ))
            .await;

        assert_eq!(out, [99, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 100]);
        assert_eq!(ret, "foo");
    })
}

#[test]
fn transform_flow() {
    async_test(async {
        fn check_send<T: Send>(x: T) -> T {
            x
        }

        let f1 = check_send(flow!(|| {
            for i in 1..=5 {
                next!(i);
            }
            "foo"
        }));

        let f2 = f1.then(transform_each!(|x| {
            next!(x * 2);
            sleep(10).await;
            next!(x * 2 + 1);
        }));

        let mut out = Vec::new();

        let ret = f2
            .then(for_each!(
                #[capture(ref mut out)]
                |x| {
                    out.push(x);
                },
            ))
            .await;

        assert_eq!(out, [2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(ret, "foo");
    })
}

#[test]
fn try_transform_flow() {
    async_test(async {
        fn check_send<T: Send>(x: T) -> T {
            x
        }

        let f1 = check_send(flow!(|| {
            for i in 1..10 {
                next!(i);
            }
            "foo"
        }));

        let f2 = f1.then(try_transform_each!(|x| {
            next!(x * 2);
            sleep(10).await;
            next!(x * 2 + 1);
            if x == 5 {
                Err("bar")
            } else {
                Ok(())
            }
        }));

        let mut out = Vec::new();

        let ret = f2
            .then(for_each!(
                #[capture(ref mut out)]
                |x| {
                    out.push(x);
                },
            ))
            .await;

        assert_eq!(out, [2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(ret, Err("bar"));
    })
}

#[test]
fn flow_from_iter() {
    async_test(async {
        let out = [1, 2, 3]
            .into_flow()
            .then(transform_each!(|x| {
                next!(x * 2 - 1);
                next!(x * 2);
            }))
            .then(Vec::from_flow)
            .await;

        assert_eq!(out, [1, 2, 3, 4, 5, 6])
    });
}

#[test]
fn flow_flatten() {
    async_test(async {
        let out = flow!(|| {
            next!(Either::Left([1, 2]));
            next!(Either::Right([3, 4, 5]));
            next!(Either::Left([6, 7]));
        })
        .then(flatten())
        .then(Vec::from_flow)
        .await;
        assert_eq!(out, [1, 2, 3, 4, 5, 6, 7])
    });
}

#[test]
fn flow_concat_map() {
    async_test(async {
        let out = [1, 2, 3]
            .into_flow()
            .then(concat_map!(|x: i32| [x * 2 - 1, x * 2]))
            .then(Vec::from_flow)
            .await;

        assert_eq!(out, [1, 2, 3, 4, 5, 6]);
    });
}

#[test]
fn flow_filter() {
    async_test(async {
        let out = flow_of![1, 2, 3, 4, 5, 6]
            .then(filter!(|x| x % 2 == 0))
            .then(Vec::from_flow)
            .await;

        assert_eq!(out, [2, 4, 6]);
    });
}

#[test]
fn flow_zip() {
    async_test(async {
        let out = zip(
            flow_of![1, 2, 3].then(delay_each(Duration::from_millis(3))),
            flow_of!["a", "b", "c", "d"].then(delay_each(Duration::from_millis(8))),
        )
        .then(Vec::from_flow)
        .await;

        assert_eq!(out, [(1, "a"), (2, "b"), (3, "c")]);
    });
}
