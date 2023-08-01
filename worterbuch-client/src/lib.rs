pub mod buffer;
pub mod config;
pub mod error;
mod ws;

use error::SubscriptionError;
use serde::{de::DeserializeOwned, Serialize};
pub use worterbuch_common::*;
pub use ws::*;

use async_stream::stream;
use futures_core::stream::Stream;
use std::sync::{Arc, Mutex};
use tokio::sync::{
    broadcast::{self},
    mpsc::{self, UnboundedSender},
};
use worterbuch_common::{
    error::{ConnectionError, ConnectionResult, WorterbuchError},
    ClientMessage as CM, Get, KeyValuePairs, PGet, PSubscribe, ServerMessage as SM, Set, Subscribe,
    Value,
};

#[derive(Debug, Clone)]
pub struct Connection {
    cmd_tx: UnboundedSender<CM>,
    result_tx: broadcast::Sender<SM>,
    counter: Arc<Mutex<u64>>,
    stop_tx: mpsc::Sender<()>,
}

impl Connection {
    pub fn new(
        cmd_tx: UnboundedSender<CM>,
        result_tx: broadcast::Sender<SM>,
        stop_tx: mpsc::Sender<()>,
    ) -> Self {
        Self {
            cmd_tx,
            result_tx,
            counter: Arc::new(Mutex::new(1)),
            stop_tx,
        }
    }

    pub fn set_value(&mut self, key: String, value: Value) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::Set(Set {
            transaction_id: i,
            key,
            value,
        }))?;
        Ok(i)
    }

    pub fn set<T: Serialize>(&mut self, key: String, value: &T) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        let value = serde_json::to_value(value)?;
        self.cmd_tx.send(CM::Set(Set {
            transaction_id: i,
            key,
            value,
        }))?;
        Ok(i)
    }

    pub fn publish_value(&mut self, key: String, value: Value) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::Publish(Publish {
            transaction_id: i,
            key,
            value,
        }))?;
        Ok(i)
    }

    pub fn publish<T: Serialize>(
        &mut self,
        key: String,
        value: &T,
    ) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        let value = serde_json::to_value(value)?;
        self.cmd_tx.send(CM::Publish(Publish {
            transaction_id: i,
            key,
            value,
        }))?;
        Ok(i)
    }

    pub fn get_async(&mut self, key: String) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::Get(Get {
            transaction_id: i,
            key,
        }))?;
        Ok(i)
    }

    pub fn pget_async(&mut self, request_pattern: String) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::PGet(PGet {
            transaction_id: i,
            request_pattern,
        }))?;
        Ok(i)
    }

    pub fn delete_async(&mut self, key: String) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::Delete(Delete {
            transaction_id: i,
            key,
        }))?;
        Ok(i)
    }

    pub fn pdelete_async(&mut self, request_pattern: String) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::PDelete(PDelete {
            transaction_id: i,
            request_pattern,
        }))?;
        Ok(i)
    }

    pub fn subscribe_async(&mut self, key: String) -> ConnectionResult<TransactionId> {
        self.do_subscribe_async(key, false)
    }

    pub fn unsubscribe_async(&mut self, transaction_id: TransactionId) -> ConnectionResult<()> {
        self.do_unsubscribe_async(transaction_id)
    }

    pub fn subscribe_unique_async(&mut self, key: String) -> ConnectionResult<TransactionId> {
        self.do_subscribe_async(key, true)
    }

    fn do_subscribe_async(&mut self, key: String, unique: bool) -> Result<u64, ConnectionError> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::Subscribe(Subscribe {
            transaction_id: i,
            key,
            unique,
        }))?;
        Ok(i)
    }

    fn do_unsubscribe_async(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<(), ConnectionError> {
        self.cmd_tx
            .send(CM::Unsubscribe(Unsubscribe { transaction_id }))?;
        Ok(())
    }

    pub fn subscribe_ls_async(
        &mut self,
        parent: Option<String>,
    ) -> ConnectionResult<TransactionId> {
        self.do_subscribe_ls_async(parent)
    }

    pub fn unsubscribe_ls_async(&mut self, transaction_id: TransactionId) -> ConnectionResult<()> {
        self.do_unsubscribe_ls_async(transaction_id)
    }

    fn do_subscribe_ls_async(&mut self, parent: Option<String>) -> Result<u64, ConnectionError> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::SubscribeLs(SubscribeLs {
            transaction_id: i,
            parent,
        }))?;
        Ok(i)
    }

    fn do_unsubscribe_ls_async(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<(), ConnectionError> {
        self.cmd_tx
            .send(CM::UnsubscribeLs(UnsubscribeLs { transaction_id }))?;
        Ok(())
    }

    pub fn psubscribe_async(&mut self, request_pattern: String) -> ConnectionResult<TransactionId> {
        self.do_psubscribe_async(request_pattern, false)
    }

    pub fn psubscribe_unique_async(
        &mut self,
        request_pattern: String,
    ) -> ConnectionResult<TransactionId> {
        self.do_psubscribe_async(request_pattern, true)
    }

    fn do_psubscribe_async(
        &mut self,
        request_pattern: String,
        unique: bool,
    ) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::PSubscribe(PSubscribe {
            transaction_id: i,
            request_pattern,
            unique,
        }))?;
        Ok(i)
    }

    pub fn ls_async(&mut self, parent: Option<Key>) -> ConnectionResult<TransactionId> {
        let i = self.inc_counter();
        self.cmd_tx.send(CM::Ls(Ls {
            transaction_id: i,
            parent,
        }))?;
        Ok(i)
    }

    pub fn responses(&mut self) -> broadcast::Receiver<SM> {
        self.result_tx.subscribe()
    }

    pub async fn get_value(&mut self, key: String) -> ConnectionResult<Value> {
        let mut subscr = self.responses();

        let i = self.inc_counter();
        self.cmd_tx.send(CM::Get(Get {
            transaction_id: i,
            key,
        }))?;

        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::State(state) => match state.event {
                                StateEvent::KeyValue(key_value) => return Ok(key_value.value),
                                StateEvent::Deleted(_) => {
                                    return Err(ConnectionError::WorterbuchError(WorterbuchError::InvalidServerResponse("a delete event is not a valid response for a get request".to_owned())))
                                }
                            },
                            SM::Err(msg) => {
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ));
                            }
                            _ => { /* ignore */ }
                        }
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn ls(&mut self, parent: Option<Key>) -> ConnectionResult<Vec<RegularKeySegment>> {
        let mut subscr = self.responses();

        let i = self.inc_counter();
        self.cmd_tx.send(CM::Ls(Ls {
            transaction_id: i,
            parent,
        }))?;

        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::LsState(state) => return Ok(state.children),
                            SM::Err(msg) => {
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ));
                            }
                            _ => { /* ignore */ }
                        }
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn get<T: DeserializeOwned>(&mut self, key: String) -> ConnectionResult<T> {
        let mut subscr = self.responses();

        let i = self.inc_counter();
        self.cmd_tx.send(CM::Get(Get {
            transaction_id: i,
            key,
        }))?;

        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::State(state) => match deserialize_state_con(state) {
                                Ok(Some(value)) => return Ok(value),
                                Err(e) => return Err(e),
                                Ok(None) => return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::InvalidServerResponse(
                                        "a get request must not be answered with a delete event"
                                            .to_owned(),
                                    ),
                                )),
                            },
                            SM::Err(msg) => {
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ))
                            }
                            _ => { /* ignore */ }
                        }
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn pget_values(
        &mut self,
        request_pattern: String,
    ) -> ConnectionResult<KeyValuePairs> {
        let mut subscr = self.responses();

        let i = self.inc_counter();
        self.cmd_tx.send(CM::PGet(PGet {
            transaction_id: i,
            request_pattern,
        }))?;

        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::PState(pstate) => {
                                match pstate.event {
                                    PStateEvent::KeyValuePairs(key_value_pairs) => return Ok(key_value_pairs),
                                    PStateEvent::Deleted(_) => {
                                        return Err(ConnectionError::WorterbuchError(WorterbuchError::InvalidServerResponse("a delte event is not a valid response for a pget request".to_owned())))
                                    },
                                }
                            },
                            SM::Err(msg) => {
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ))
                            }
                            msg => {
                                log::warn!("received unrelated msg with pget tid {tid}: {msg:?}")
                            }
                        }
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn pget<T: DeserializeOwned>(
        &mut self,
        request_pattern: String,
    ) -> ConnectionResult<TypedKeyValuePairs<T>> {
        let mut subscr = self.responses();

        let i = self.inc_counter();
        self.cmd_tx.send(CM::PGet(PGet {
            transaction_id: i,
            request_pattern,
        }))?;

        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::PState(pstate) => match pstate.event {
                                PStateEvent::KeyValuePairs(kvps) => return deserialize_pstate_con(kvps),
                                PStateEvent::Deleted(_) => return Err(ConnectionError::WorterbuchError(WorterbuchError::InvalidServerResponse("a delte event is not a valid response for a pget request".to_owned()))),
                            },
                            SM::Err(msg) => {
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ))
                            }
                            msg => {
                                log::warn!("received unrelated msg with pget tid {tid}: {msg:?}")
                            }
                        }
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn subscribe_values(
        &mut self,
        key: String,
    ) -> ConnectionResult<impl Stream<Item = Result<Option<Value>, SubscriptionError>>> {
        self.do_subscribe_values(key, false).await
    }

    pub async fn subscribe<T: DeserializeOwned>(
        &mut self,
        key: String,
    ) -> ConnectionResult<impl Stream<Item = Result<Option<T>, SubscriptionError>>> {
        self.do_subscribe(key, false).await
    }

    pub async fn subscribe_unique_values(
        &mut self,
        key: String,
    ) -> ConnectionResult<impl Stream<Item = Result<Option<Value>, SubscriptionError>>> {
        self.do_subscribe_values(key, true).await
    }

    pub async fn subscribe_unique<T: DeserializeOwned>(
        &mut self,
        key: String,
    ) -> ConnectionResult<impl Stream<Item = Result<Option<T>, SubscriptionError>>> {
        self.do_subscribe(key, true).await
    }

    async fn do_subscribe_values(
        &mut self,
        key: String,
        unique: bool,
    ) -> ConnectionResult<impl Stream<Item = Result<Option<Value>, SubscriptionError>>> {
        let mut subscr = self.responses();
        let i = self.inc_counter();
        let owned_key = key.clone();
        self.cmd_tx.send(CM::Subscribe(Subscribe {
            transaction_id: i,
            key,
            unique,
        }))?;
        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::Err(msg) => {
                                log::warn!("subscription {tid} to key {owned_key} failed");
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ));
                            }
                            SM::Ack(_) => {
                                return Ok(stream! {
                                    loop {
                                        match subscr.recv().await{
                                            Ok(msg) =>{let tid = msg.transaction_id();
                                                if tid == i {
                                                    match msg {
                                                        SM::State(state) => {
                                                            match state.event {
                                                                StateEvent::KeyValue(kv) =>  yield Ok(Some(kv.value)),
                                                                StateEvent::Deleted(_) =>  yield Ok(None),
                                                            }
                                                        }
                                                        SM::Err(err) => {
                                                            log::error!("Error in subscription of {owned_key}: {err:?}");
                                                            yield Err(SubscriptionError::ServerError(err));
                                                                break;
                                                        }
                                                        msg => log::warn!(
                                                            "received unrelated msg with subscription tid {tid}: {msg:?}"
                                                        ),
                                                    }
                                                }}
                                            Err(e) => {
                                                yield Err(SubscriptionError::RecvError(e));
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            msg => log::warn!(
                                "received unrelated msg with subscription tid {tid}: {msg:?}"
                            ),
                        }
                        break;
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(ConnectionError::WorterbuchError(
            WorterbuchError::NotSubscribed,
        ))
    }

    async fn do_subscribe<T: DeserializeOwned>(
        &mut self,
        key: String,
        unique: bool,
    ) -> ConnectionResult<impl Stream<Item = Result<Option<T>, SubscriptionError>>> {
        let mut subscr = self.responses();
        let i = self.inc_counter();
        let owned_key = key.clone();
        self.cmd_tx.send(CM::Subscribe(Subscribe {
            transaction_id: i,
            key,
            unique,
        }))?;
        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::Err(msg) => {
                                log::warn!("subscription {tid} to key {owned_key} failed");
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ));
                            }
                            SM::Ack(_) => {
                                return Ok(stream! {
                                    loop {
                                        match subscr.recv().await{
                                            Ok(msg) =>{let tid = msg.transaction_id();
                                                if tid == i {
                                                    match msg {
                                                        SM::State(state) => {
                                                            yield deserialize_state_sub::<T>(state);
                                                        }
                                                        SM::Err(err) => {
                                                            log::error!("Error in subscription of {owned_key}: {err:?}");
                                                            yield Err(SubscriptionError::ServerError(err));
                                                                break;
                                                        }
                                                        msg => log::warn!(
                                                            "received unrelated msg with subscription tid {tid}: {msg:?}"
                                                        ),
                                                    }
                                                }}
                                            Err(e) => {
                                                yield Err(SubscriptionError::RecvError(e));
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            msg => log::warn!(
                                "received unrelated msg with subscription tid {tid}: {msg:?}"
                            ),
                        }
                        break;
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(ConnectionError::WorterbuchError(
            WorterbuchError::NotSubscribed,
        ))
    }

    pub async fn subscribe_ls(
        &mut self,
        parent: Option<String>,
    ) -> ConnectionResult<impl Stream<Item = Result<Vec<RegularKeySegment>, SubscriptionError>>>
    {
        self.do_subscribe_ls(parent).await
    }

    async fn do_subscribe_ls(
        &mut self,
        parent: Option<String>,
    ) -> ConnectionResult<impl Stream<Item = Result<Vec<RegularKeySegment>, SubscriptionError>>>
    {
        let mut subscr = self.responses();
        let i = self.inc_counter();
        let owned_parent = parent.clone();
        self.cmd_tx.send(CM::SubscribeLs(SubscribeLs {
            transaction_id: i,
            parent,
        }))?;
        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::Err(msg) => {
                                log::warn!("subscription {tid} to key {owned_parent:?} failed");
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ));
                            }
                            SM::Ack(_) => {
                                return Ok(stream! {
                                    loop {
                                        match subscr.recv().await{
                                            Ok(msg) =>{let tid = msg.transaction_id();
                                                if tid == i {
                                                    match msg {
                                                        SM::LsState(state) => {
                                                            yield Ok(state.children);
                                                        }
                                                        SM::Err(err) => {
                                                            log::error!("Error in ls subscription of {owned_parent:?}: {err:?}");
                                                            yield Err(SubscriptionError::ServerError(err));
                                                                break;
                                                        }
                                                        msg => log::warn!(
                                                            "received unrelated msg with subscription tid {tid}: {msg:?}"
                                                        ),
                                                    }
                                                }}
                                            Err(e) => {
                                                yield Err(SubscriptionError::RecvError(e));
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            msg => log::warn!(
                                "received unrelated msg with subscription tid {tid}: {msg:?}"
                            ),
                        }
                        break;
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(ConnectionError::WorterbuchError(
            WorterbuchError::NotSubscribed,
        ))
    }

    pub async fn psubscribe_values(
        &mut self,
        request_pattern: String,
    ) -> ConnectionResult<impl Stream<Item = Result<PStateEvent, SubscriptionError>>> {
        self.do_psubscribe_values(request_pattern, false).await
    }

    pub async fn psubscribe<T: DeserializeOwned>(
        &mut self,
        request_pattern: String,
    ) -> ConnectionResult<impl Stream<Item = Result<TypedStateEvent<T>, SubscriptionError>>> {
        self.do_psubscribe::<T>(request_pattern, false).await
    }

    pub async fn psubscribe_unique_values(
        &mut self,
        request_pattern: String,
    ) -> ConnectionResult<impl Stream<Item = Result<PStateEvent, SubscriptionError>>> {
        self.do_psubscribe_values(request_pattern, true).await
    }

    pub async fn psubscribe_unique<T: DeserializeOwned>(
        &mut self,
        request_pattern: String,
    ) -> ConnectionResult<impl Stream<Item = Result<TypedStateEvent<T>, SubscriptionError>>> {
        self.do_psubscribe(request_pattern, true).await
    }

    async fn do_psubscribe_values(
        &mut self,
        request_pattern: String,
        unique: bool,
    ) -> ConnectionResult<impl Stream<Item = Result<PStateEvent, SubscriptionError>>> {
        let mut subscr = self.responses();
        let i = self.inc_counter();
        let owned_pattern = request_pattern.clone();
        self.cmd_tx.send(CM::PSubscribe(PSubscribe {
            transaction_id: i,
            request_pattern,
            unique,
        }))?;
        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::Err(msg) => {
                                log::warn!("subscription {tid} to pattern {owned_pattern} failed");
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ));
                            }
                            SM::Ack(_) => {
                                return Ok(stream! {
                                    loop {
                                        match subscr.recv().await {
                                            Ok(msg) => {
                                                let tid = msg.transaction_id();
                                        if tid == i {
                                            match msg {
                                                SM::PState(pstate) => {
                                                    yield Ok(pstate.event)
                                                }
                                                SM::Err(err) => {
                                                    log::error!("Error in subscription of {owned_pattern}: {err:?}");
                                                    yield Err(SubscriptionError::ServerError(err));
                                                    break;
                                                }
                                                _ => { /* ignore */ }
                                            }
                                        }
                                            },
                                            Err(e) => {
                                                log::error!("Error receiving message: {e}");
                                                yield Err(SubscriptionError::RecvError(e));
                                                break;
                                            }
                                        }
                                        // TODO time out
                                    }
                                });
                            }
                            msg => log::warn!(
                                "received unrelated msg with subscription tid {tid}: {msg:?}"
                            ),
                        }
                        break;
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(ConnectionError::WorterbuchError(
            WorterbuchError::NotSubscribed,
        ))
    }

    async fn do_psubscribe<T: DeserializeOwned>(
        &mut self,
        request_pattern: String,
        unique: bool,
    ) -> ConnectionResult<impl Stream<Item = Result<TypedStateEvent<T>, SubscriptionError>>> {
        let mut subscr = self.responses();
        let i = self.inc_counter();
        let owned_pattern = request_pattern.clone();
        self.cmd_tx.send(CM::PSubscribe(PSubscribe {
            transaction_id: i,
            request_pattern,
            unique,
        }))?;
        loop {
            match subscr.recv().await {
                Ok(msg) => {
                    let tid = msg.transaction_id();
                    if tid == i {
                        match msg {
                            SM::Err(msg) => {
                                log::warn!("subscription {tid} to pattern {owned_pattern} failed");
                                return Err(ConnectionError::WorterbuchError(
                                    WorterbuchError::ServerResponse(msg),
                                ));
                            }
                            SM::Ack(_) => {
                                return Ok(stream! {
                                    loop {
                                        match subscr.recv().await {
                                            Ok(msg) => {
                                                let tid = msg.transaction_id();
                                        if tid == i {
                                            match msg {
                                                SM::PState(pstate) => {
                                                    match deserialize_pstate_sub(pstate) {
                                                        Ok(events) => {
                                                            for event in events {
                                                                yield Ok(event);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            yield Err(e);
                                                        }
                                                    }
                                                }
                                                SM::Err(err) => {
                                                    log::error!("Error in subscription of {owned_pattern}: {err:?}");
                                                    yield Err(SubscriptionError::ServerError(err));
                                                    break;
                                                }
                                                _ => { /* ignore */ }
                                            }
                                        }
                                            },
                                            Err(e) => {
                                                log::error!("Error receiving message: {e}");
                                                yield Err(SubscriptionError::RecvError(e));
                                                break;
                                            }
                                        }
                                        // TODO time out
                                    }
                                });
                            }
                            msg => log::warn!(
                                "received unrelated msg with subscription tid {tid}: {msg:?}"
                            ),
                        }
                        break;
                    }
                    // TODO time out
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(ConnectionError::WorterbuchError(
            WorterbuchError::NotSubscribed,
        ))
    }

    pub fn send(&mut self, msg: CM) -> ConnectionResult<()> {
        self.cmd_tx.send(msg)?;
        Ok(())
    }

    fn inc_counter(&mut self) -> u64 {
        let mut counter = self.counter.lock().expect("counter mutex poisoned");
        let current = *counter;
        *counter += 1;
        current
    }

    pub async fn close(&self) {
        self.stop_tx.send(()).await.ok();
    }
}

fn deserialize_state_con<T: DeserializeOwned>(state: State) -> Result<Option<T>, ConnectionError> {
    let typed: TypedStateEvent<T> = state.event.try_into()?;
    Ok(typed.into())
}

fn deserialize_pstate_con<T: DeserializeOwned>(
    kvps: KeyValuePairs,
) -> Result<TypedKeyValuePairs<T>, ConnectionError> {
    let mut typed = TypedKeyValuePairs::new();
    for kvp in kvps {
        typed.push(kvp.try_into()?);
    }
    Ok(typed)
}

fn deserialize_state_sub<T: DeserializeOwned>(
    state: State,
) -> Result<Option<T>, SubscriptionError> {
    let typed: TypedStateEvent<T> = state.event.try_into()?;
    Ok(typed.into())
}

fn deserialize_pstate_sub<T: DeserializeOwned>(
    pstate: PState,
) -> Result<TypedStateEvents<T>, SubscriptionError> {
    Ok(pstate.try_into()?)
}
