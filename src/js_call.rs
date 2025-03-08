use std::sync::Arc;

use futures::Future;
use napi::{
    bindgen_prelude::{FromNapiValue, JsValuesTupleIntoVec, Promise},
    threadsafe_function::{ThreadsafeFunction, UnknownReturnValue},
    Either,
};
use tracing::{debug, error};

use std::borrow::Cow;

use regex::Regex;
use std::sync::LazyLock;

static MODULE_MATCHER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:\w+::)").unwrap());

pub fn pretty_type_name<T: ?Sized>() -> Cow<'static, str> {
    let type_name = std::any::type_name::<T>();
    prettify_type_name(type_name)
}

fn prettify_type_name(name: &str) -> Cow<str> {
    MODULE_MATCHER_RE.replace_all(name, "")
}

/// `JsCallback`  is a type alias for `ThreadsafeFunction`. It represents a JavaScript function that passed to Rust side.
/// Related concepts are complex, so we use `JsCallback` to simplify the mental model. For details, please refer to:
/// - https://napi.rs/docs/compat-mode/concepts/thread-safe-function.en
/// - https://github.com/napi-rs/napi-rs
///
/// ## Examples
///
/// You will notice that I place Js under the Rust type, because the Js type is generated to fit the Rust type.
/// We need write correct Js types to satisfy the Rust type not the other way around.
///
/// ### Sync
///
/// - Rust: `JsCallback<FnArgs<(String, i32)>, i32>`
/// - Js: `(a: string, b: number) => number`
///
/// For `Option<T>` in params position, when you pass `None` in Rust, it will be `null` in Js. However, NAPI-RS generates
/// `undefined | null | T` as the Js type. So, we introduce `Nullable<T>` in Js to simplify the writing.
///
/// One tricky part for `Option<T>` in return position is that the corresponding Js type is `T | null | undefined` will make
/// Ts compiler force you to write `return` statement explicitly. To avoid this, we introduce `VoidNullable<T>` in Js. It will
/// expand to `T | null | undefined | void`.
///
/// - Rust: `JsCallback<FnArgs<(Option<String>, i32)>, Option<i32>>`
/// - Js: `(a: string | null | undefined, b: number) => number | null | undefined | void`
/// - Js(Simplified): `(a: Nullable<string>, b: number) => VoidNullable<number>`
///
/// ### Async
///
/// For async functions in Js, remember these functions are also sync functions that return a `Promise<T>`. What's good is that
/// Ts compiler force you to add `Promise<T>` in the return type when you write async functions. So, you could consider they are
/// just sync functions that return `Promise<T>`.
///
/// - Rust: `JsCallback<FnArgs<(String, i32)>, Promise<i32>>`
/// - Js: `(a: string, b: number) => Promise<number>`
///
/// ---
///
/// - Rust: `JsCallback<FnArgs<(Option<String>, i32)>, Promise<Option<i32>>>`
/// - Js: `(a: string | null | undefined, b: number) => Promise<number | null | undefined | void>`
/// - Js(Simplified): `(a: Nullable<string>, b: number) => Promise<VoidNullable<number>>`
///
/// ### MaybeAsync
///
/// Sometimes we want to accept functions that could be sync or async, as we just said above, it's actually just a sync function
/// that returns `Promise<T> | T`. This pattern is very common in NAPI-RS, so we introduce `MaybeAsyncJsCallback` to simplify it.
///
/// Notice the order matters for rust types `Either<Promise<T>, T>` and `Either<T, Promise<T>>`. Always use `Either<Promise<T>, T>`.
///
/// - Rust: `JsCallback<FnArgs<(String, i32), Either<Promise<i32>>, i32>>`
/// - Rust(Simplified): `MaybeAsyncJsCallback<FnArgs<(String, i32)>, i32>`
/// - Js: `(a: string, b: number) => Promise<number> | number`
/// - Js(Simplified): `(a: string, b: number) => MaybePromise<number>`
///
/// ---
///
/// - Rust: `JsCallback<FnArgs<(Option<String>, i32), Either<Promise<Option<i32>>>, Option<i32>>`
/// - Rust(Simplified): `MaybeAsyncJsCallback<FnArgs<(Option<String>, i32)>, Option<i32>>`
/// - Js: `(a: string | null | undefined, b: number) => Promise<number | null | undefined | void> | number | null | undefined | void`
/// - Js(Simplified): `(a: Nullable<string>, b: number) => MaybePromise<VoidNullable<number>>`
pub type JsCallback<Args, Ret> =
    Arc<ThreadsafeFunction<Args, Either<Ret, UnknownReturnValue>, Args, false, true>>;

/// Shortcut for `JsCallback<FnArgs<..., Either<Promise<Ret>, Ret>>`, which could be simplified to `MaybeAsyncJsCallback<...>, Ret>`.
pub type MaybeAsyncJsCallback<Args, Ret> = Arc<
    ThreadsafeFunction<
        Args,
        Either<Either<Promise<Ret>, Ret>, UnknownReturnValue>,
        Args,
        false,
        true,
    >,
>;

pub trait JsCallbackExt<Args, Ret> {
    fn invoke_async(&self, args: Args) -> impl Future<Output = Result<Ret, napi::Error>> + Send;
}

impl<Args, Ret> JsCallbackExt<Args, Ret> for JsCallback<Args, Ret>
where
    Args: 'static + Send + JsValuesTupleIntoVec,
    Ret: 'static + Send + FromNapiValue,
    napi::Either<Ret, UnknownReturnValue>: FromNapiValue,
{
    async fn invoke_async(&self, args: Args) -> Result<Ret, napi::Error> {
        match self.call_async(args).await? {
            Either::A(ret) => Ok(ret),
            Either::B(_unknown) => {
                // TODO: should provide more information about the unknown return value
                let js_type = "unknown";

                Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    format!("UNKNOWN_RETURN_VALUE. Cannot convert {js_type} to `` in ",),
                ))
            }
        }
    }
}

pub trait MaybeAsyncJsCallbackExt<Args, Ret> {
    /// Call Js function asynchronously in rust. If the Js function returns `Promise<T>`, it will unwrap/await the promise and return `T`.
    fn await_call(&self, args: Args) -> impl Future<Output = Result<Ret, napi::Error>> + Send;
}
impl<Args, Ret> MaybeAsyncJsCallbackExt<Args, Ret> for JsCallback<Args, Either<Promise<Ret>, Ret>>
where
    Args: 'static + Send + JsValuesTupleIntoVec,
    Ret: 'static + Send + FromNapiValue,
    napi::Either<napi::Either<Promise<Ret>, Ret>, UnknownReturnValue>: FromNapiValue,
{
    #[allow(clippy::manual_async_fn)]
    fn await_call(&self, args: Args) -> impl Future<Output = Result<Ret, napi::Error>> + Send {
        async move {
            println!("Calling JavaScript function with args");
            match self.call_async(args).await {
                Ok(result) => match result {
                    Either::A(Either::A(promise)) => {
                        println!("JavaScript function returned a promise, awaiting the promise");
                        promise.await
                    }
                    Either::A(Either::B(ret)) => {
                        println!("JavaScript function returned a value");
                        Ok(ret)
                    }
                    Either::B(_unknown) => {
                        let js_type = "unknown";
                        let expected_rust_type = pretty_type_name::<Ret>();
                        println!("Unknown return value from JavaScript function");

                        Err(napi::Error::new(
                            napi::Status::InvalidArg,
                            format!(
                "UNKNOWN_RETURN_VALUE. Cannot convert {js_type} to `{expected_rust_type}` in {}.",
                pretty_type_name::<Self>(),
              ),
                        ))
                    }
                },
                Err(e) => {
                    println!("Error calling JavaScript function: {:?}", e);
                    Err(e)
                }
            }
        }
    }
}
