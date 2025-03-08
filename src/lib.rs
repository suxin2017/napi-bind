pub mod js_call;

use std::{fs, future::Future, sync::Arc, thread};

use js_call::MaybeAsyncJsCallbackExt;
use napi::{
    bindgen_prelude::{FnArgs, Promise},
    threadsafe_function::{ThreadsafeFunction, UnknownReturnValue},
    Either,
};
use napi_derive::napi;

pub type MaybeAsyncJsCallback<Args, Ret> = Arc<
    ThreadsafeFunction<
        Args,
        Either<Either<Promise<Ret>, Ret>, UnknownReturnValue>,
        Args,
        false,
        true,
    >,
>;

use std::fmt::Display;

#[derive(Debug)]
pub enum WatcherEvent {
    Close,
    Event(BundleEvent),
    ReStart,
}

impl Display for WatcherEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WatcherEvent::Close => write!(f, "close"),
            WatcherEvent::Event(_) => write!(f, "event"),
            WatcherEvent::ReStart => write!(f, "restart"),
        }
    }
}

#[derive(Debug)]
pub enum BundleEvent {
    Start,
    BundleStart,
    End,
}

impl Display for BundleEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BundleEvent::Start => write!(f, "START"),
            BundleEvent::BundleStart => write!(f, "BUNDLE_START"),
            BundleEvent::End => write!(f, "END"),
        }
    }
}

#[derive(Debug)]
pub struct BundleEndEventData {
    pub output: String,
    pub duration: u32,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum WatcherChangeKind {
    Create,
    Update,
    Delete,
}
#[napi]
pub struct BindingWatcherChangeData {
    pub path: String,
    pub kind: String,
}
#[napi]
pub struct BindingBundleEndEventData {
    pub output: String,
    pub duration: u32,
}

impl Display for WatcherChangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WatcherChangeKind::Create => write!(f, "create"),
            WatcherChangeKind::Update => write!(f, "update"),
            WatcherChangeKind::Delete => write!(f, "delete"),
        }
    }
}

#[napi]
pub struct BindingWatcherEvent {
    inner: WatcherEvent,
}
#[napi]
pub struct BindingError {
    pub kind: String,
    pub message: String,
}

#[napi]
impl BindingWatcherEvent {
    pub fn new(inner: WatcherEvent) -> Self {
        Self { inner }
    }

    #[napi]
    pub fn event_kind(&self) -> String {
        self.inner.to_string()
    }

    #[napi]
    pub fn watch_change_data(&self) -> BindingWatcherChangeData {
        match &self.inner {
            _ => {
                unreachable!("Expected WatcherEvent::Change")
            }
        }
    }

    #[napi]
    pub fn bundle_end_data(&self) -> BindingBundleEndEventData {
        match &self.inner {
            _ => {
                unreachable!("Expected WatcherEvent::Event(BundleEventKind::BundleEnd)")
            }
        }
    }

    #[napi]
    pub fn bundle_event_kind(&self) -> String {
        match &self.inner {
            WatcherEvent::Event(kind) => kind.to_string(),
            _ => {
                unreachable!("Expected WatcherEvent::Event")
            }
        }
    }

    #[napi]
    pub fn errors(&mut self) -> Vec<napi::Either<napi::JsError, BindingError>> {
        unimplemented!("errors")
    }
}

// use tokio_with_wasm::alias as tokio;

use tokio::task::{spawn, spawn_blocking, yield_now, JoinSet};
use tokio_with_wasm::alias as tokio;

#[napi]
pub struct BindingWatcher {}

#[napi]
impl BindingWatcher {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        Ok(Self {})
    }

    #[tracing::instrument(level = "debug", skip_all)]
    #[napi(ts_args_type = "listener: (data: BindingWatcherEvent) => void")]
    pub async fn start(&self, listener: MaybeAsyncJsCallback<(), ()>) -> napi::Result<()> {
        let f = async move {
            println!("why here is not running");

            println!("async call");
            if let Err(e) = listener.await_call(()).await {
                println!("async watcher listener error: {:?}", e);
                eprintln!("async watcher listener error: {e:?}");
            }
            println!("async no lock");
        };

        spawn(f);

        tokio_with_wasm::alias::spawn(async move {
            println!("why here is not running1");
        })
        .await;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    #[napi(ts_args_type = "listener: (data: BindingWatcherEvent) => void")]
    pub async fn loop_spawn(&self, listener: MaybeAsyncJsCallback<(), ()>) -> napi::Result<()> {
        let f = async move {
            println!("why here is not running");

            println!("async call");
            if let Err(e) = listener.await_call(()).await {
                println!("async watcher listener error: {:?}", e);
                eprintln!("async watcher listener error: {e:?}");
            }
            println!("async no lock");
        };

        spawn(f);

        tokio_with_wasm::alias::spawn(async move {
            println!("why here is not running1");
            loop {}
        })
        .await;
        Ok(())
    }
}
