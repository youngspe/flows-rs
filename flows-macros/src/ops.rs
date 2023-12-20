#[doc(hidden)]
#[macro_export]
macro_rules! _try_for_each {
    {
        $(#[capture $cap:tt])*
        $($move:ident)? |$input:tt : $input_ty:ty| -> $resume:ty $rest:block,
        $init:expr
     } => {{
        $crate::capture_outer! { [$($cap)*] }
        $crate::macro_utils::_flow_op::<
            $input_ty,
            $resume,
            _, _, _
        >($($move)? |src| $crate::flows::ops::try_for_each_init(src, $crate::_collector! {
            $(#[capture { inner: $cap }])*
            $($move)? |$input: $input_ty| -> $resume $rest
        }, $init))
    }};
    {
        $(#[capture $cap:tt])*
        $($move:ident)? |$input:tt : $input_ty:ty| -> $resume:ty $rest:block
    } => {{
        $crate::capture_outer! { [$($cap)*] }
        $crate::macro_utils::_flow_op::<
            $input_ty,
            $resume,
            _, _, _
        >($($move)? |src| $crate::flows::ops::try_for_each(src, $crate::_collector! {
            $(#[capture { inner: $cap }])*
            $($move)? |$input: $input_ty| -> $resume $rest
        }))
    }};
}

#[macro_export]
macro_rules! try_for_each {
    { $($x:tt)* } => {
        $crate::normalize_op_input! { ($crate::_try_for_each) {} $($x)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _for_each {
    {
        $(#[capture $cap:tt])*
        $($move:ident)? |$input:tt : $input_ty:ty| -> $resume:ty $rest:block,
        $init:expr
     } => {{
        $crate::capture_outer! { [$($cap)*] }
        $crate::macro_utils::_flow_op::<
            $input_ty,
            $resume,
            _, _, _
        >($($move)? |src| $crate::flows::ops::for_each_init(src, $crate::collector! {
            $(#[capture { inner: $cap }])*
            $($move)? |$input: $input_ty| -> $resume $rest
        }, $init))
    }};
    {
        $(#[capture $cap:tt])*
        $($move:ident)? |$input:tt : $input_ty:ty| -> $resume:ty $rest:block
     } => {{
        $crate::capture_outer! { [$($cap)*] }
        $crate::macro_utils::_flow_op::<
            $input_ty,
            $resume,
            _, _, _
        >($($move)? |src| $crate::flows::ops::for_each(src, $crate::collector! {
            $(#[capture { inner: $cap }])*
            $($move)? |$input: $input_ty| -> $resume $rest
        }))
    }};
}

#[macro_export]
macro_rules! for_each {
    { $($x:tt)* } => {
        $crate::normalize_op_input! { ($crate::_for_each) {} $($x)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _try_transform_each {
    {
        $(#[capture $cap:tt])*
        $($move:ident)? |$input:tt : $input_ty:ty, $($sender1:ident)? $((mut $sender2:ident))? : $sender_ty:ty| -> $resume:ty $rest:block
    } => {{
        $crate::capture_outer! { [$($cap)*] }
        $crate::macro_utils::_flow_try_transform_op::<
            $input_ty, $resume, _,
            _, _,
            _, _,
        >($($move)? |src| {
            let src = $crate::macro_utils::NotCopy(src);
            $crate::_flow! {
                $($move)? |_transform_resume: $resume, $($sender1)? $($sender2)?: $sender_ty| -> _ {
                    let src = src;
                    $crate::_try_for_each!(
                        #[capture($($sender1)? $(mut $sender2)?)]
                        $(#[capture { inner: $cap }])*
                        $($move)? |$input: $input_ty| -> $resume {
                            $rest
                        },
                        _transform_resume
                    ).execute(src.0).await
                }
            }
        })
    }};
    {
        $(#$attr:tt)*
        $($move:ident)? |$input:tt : $input_ty:ty| -> $resume:ty $rest:block
    } => {{
        $crate::_try_transform_each! {
            $(#$attr)* |$input : $input_ty, (mut _transform_sender): _| -> $resume {
                $crate::_flow_local_macros! { ($) _transform_sender, $rest }
            }
        }
    }};
}

#[macro_export]
macro_rules! try_transform_each {
    { $($x:tt)* } => {
        $crate::normalize_op_input! { ($crate::_try_transform_each) {} $($x)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _transform_each {
    {
        $(#[capture $cap:tt])*
        $($move:ident)? |$input:tt : $input_ty:ty, $($sender1:ident)? $((mut $sender2:ident))? : $sender_ty:ty| -> $resume:ty $rest:block
    } => {{
        $crate::capture_outer! { [$($cap)*] }
        $crate::macro_utils::_flow_transform_op::<
            $input_ty, $resume, _,
            _,
            _, _,
        >($($move)? |src| {
            let src = $crate::macro_utils::NotCopy(src);
            $crate::_flow! {
                $($move)? |_transform_resume: $resume, $($sender1)? $($sender2)?: $sender_ty| -> _ {
                    let src = src;
                    $crate::_for_each!(
                        #[capture($($sender1)? $(mut $sender2)?)]
                        $(#[capture { inner: $cap }])*
                        $($move)? |$input: $input_ty| -> $resume {
                            $rest
                        },
                        _transform_resume
                    ).execute(src.0).await
                }
            }
        })
    }};
    {
        $(#$attr:tt)*
        $($move:ident)? |$input:tt : $input_ty:ty| -> $resume:ty $rest:block
    } => {{
        $crate::_transform_each! {
            $(#$attr)* |$input : $input_ty, (mut _transform_sender): _| -> $resume {
                $crate::_flow_local_macros! { ($) _transform_sender, $rest }
            }
        }
    }};
}

#[macro_export]
macro_rules! transform_each {
    { $($x:tt)* } => {
        $crate::normalize_op_input! { ($crate::_transform_each) {} $($x)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _map {
    {
        $(#$attr:tt)*
        $($move:ident)? |$input:tt : $input_ty:ty| -> $output_ty:ty $rest:block
    } => {
        $crate::_transform_each! {
            $(#$attr)*
            $($move)? |$input: $input_ty, (mut _map_sender): $crate::flows::Sender<$output_ty, _>| -> _ {
                _map_sender.next(async { $rest }.await).await
            }
        }
    };
}

#[macro_export]
macro_rules! map_each {
    { $($x:tt)* } => {
        $crate::normalize_op_input! { ($crate::_map) {} $($x)* }
    };
}

#[macro_export]
macro_rules! merge_map {
    ($($x:tt)*) => {
        $crate::macro_utils::_flow_op(|src| $crate::flows::ops::merge_all().execute(
            $crate::map_each!($($x)*).execute(src))
        )
    };
}

#[macro_export]
macro_rules! concat_map {
    ($($x:tt)*) => {
        $crate::macro_utils::_flow_op(|src| $crate::flows::ops::flatten().execute(
            $crate::map_each!($($x)*).execute(src))
        )
    };
}

#[macro_export]
macro_rules! switch_map {
    ($($x:tt)*) => {
        $crate::macro_utils::_flow_op(|src| $crate::flows::ops::flatten().execute(
            $crate::map_each!($($x)*).execute($crate::flows::ops::latest().execute(src)))
        )
    };
}

#[macro_export]
macro_rules! map_return {
    ($(#$attr:tt)* $($move:ident)? |$input:pat_param| $($rest:tt)* ) => {
        $crate::map_return! { $(#$attr)* $($move)? |$input: _| $($rest)* }
    };
    {
        $(#[capture $cap:tt])*
        $($move:ident)? |$input:tt $($input2:ident)* : $input_ty:ty|
        $(-> $ret:ty)? $rest:block
        $(,)?
    } => {
        $crate::macro_utils::_flow_op(|src| $crate::flow! {
            $($attr)* #[capture(src)] #[capture $cap] $($move)? |init, mut sender| $(-> $ret)? {
                let $input $($input2)* $(: $input_ty)? = sender.next_from(src, init).await;
                $rest
            }
        })
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _filter {
    {
        $(#$attr:tt)*
        $($move:ident)? |$input:tt : $input_ty:ty| -> $output_ty:ty $rest:block
    } => {
        $crate::_transform_each! {
            $(#$attr)*
            $($move)? |filter_input: _, (mut _filter_sender): _| -> _ {
                let allowed: $output_ty = async {
                    let $input: $input_ty = &filter_input;
                    $rest
                }.await;
                if allowed {
                    _filter_sender.next(filter_input).await
                }
            }
        }
    };
}

#[macro_export]
macro_rules! filter {
    { $($x:tt)* } => {
        $crate::normalize_op_input! { ($crate::_filter) {} $($x)* }
    };
}
