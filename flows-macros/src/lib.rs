#[doc(hidden)]
pub extern crate flows_core as flows;

#[doc(hidden)]
pub mod macro_utils;
mod ops;

#[macro_export]
macro_rules! flow_of {
    ($($x:expr),* $(,)?) => {
        $crate::_flow!(|(): (), sender: _| -> _ {
            #[allow(unused_mut)]
            let mut sender = sender;
            $(sender.next($x).await;)*
        });
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _flow_local_macros {
    (($d:tt) $sender:expr, $($rest:tt)*) => {{
        #[allow(unused)]
        let ref mut sender @ $crate::flows::Sender { .. } = $sender;

        #[allow(unused)]
        macro_rules! sender {
            () => { *sender };
        }

        #[allow(unused)]
        macro_rules! next {
            ($value:expr) => {
                sender.next($value).await
            };
            () => {
                sender.next(()).await
            };
        }

        #[allow(unused)]
        macro_rules! next_from {
            ($flow:expr, $init:expr $d (,)?) => {
                sender.next_from($flow, $init).await
            };
            ($flow:expr $d (,)?) => {
                sender.next_from($flow, ::core::default::Default::default()).await
            };
        }

        $($rest)*
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! _flow {
    { $(#$attr:tt)* $($move:ident)? | | $($rest:tt)* } => {
        $crate::_flow! { $(#$attr)* $($move)? |(): ()| $($rest)* }
    };
    {
        $(#[capture $cap:tt])*
        $($move:ident)? |
            $input:tt : $input_ty:ty,
            $sender:tt : $sender_ty:ty
        | -> $ret:ty $rest:block
    } => {{
        $crate::capture_outer! { [$($cap)*] }
        $crate::flows::flow_from_fn::<_, $input_ty, $ret>($($move)? |_input: $input_ty, _sender: $crate::flows::Sender<_, $input_ty>| {
            // use NotCopy to send our arg to make sure it gets moved and not just copied when the closure isn't move.
            let _input = $crate::macro_utils::NotCopy(_input);
            let _raw_sender = _sender.into_raw();
            async $($move)? {
                let _input = _input;
                #[allow(unused_parens)]
                let $input = _input.0;

                $crate::capture_inner! { [$($cap)*] }

                let mut raw_sender = _raw_sender;
                let $sender : $sender_ty = unsafe { raw_sender.as_sender() };

                $rest
            }
        })
    }};
    {
        $(#$attr:tt)*
        $($move:ident)? |
            $input:tt : $input_ty:ty
        | -> $ret:ty $rest:block
     } => {
        $crate::_flow! { $(#$attr)* $($move)? |$input : $input_ty, (mut sender): $crate::flows::Sender<_, $input_ty>| -> $ret {
            $crate::_flow_local_macros! { ($) sender, $rest }
        } }
    }
}

#[macro_export]
macro_rules! flow {
    { $($x:tt)* } => {
        $crate::normalize_op_input! { ($crate::_flow) {} $($x)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _collector {
    ($(#$attr:tt)* $($move:ident)? |$input:tt : $input_ty:ty| -> $resume:ty $rest:block ) => {
        $crate::_flow! { $(#$attr)* $($move)? |
            (mut _collector_input): $input_ty,
            (mut _collector_sender): $crate::flows::Sender<$resume, $input_ty>
        | -> ::core::convert::Infallible {
            loop {
                let $input: $input_ty = _collector_input;
                // wrap the loop body in an async block so a return doesn't break out of the loop
                _collector_input = _collector_sender.next_await(async { $rest }).await;
            }
        } }
    };
}

#[macro_export]
macro_rules! collector {
    { $($x:tt)* } => {
        $crate::normalize_op_input! { ($crate::_collector) {} $($x)* }
    };
}
