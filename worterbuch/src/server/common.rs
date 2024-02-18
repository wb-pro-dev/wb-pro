/*
 *  Worterbuch server common module
 *
 *  Copyright (C) 2024 Michael Bachmann
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::{
    auth::{get_claims, JwtClaims},
    subscribers::SubscriptionId,
    Config, PStateAggregator,
};
use serde::Serialize;
use std::{net::SocketAddr, time::Duration};
use tokio::{
    spawn,
    sync::{
        mpsc::{self, UnboundedReceiver},
        oneshot,
    },
};
use uuid::Uuid;
use worterbuch_common::{
    error::{Context, WorterbuchError, WorterbuchResult},
    Ack, AuthenticationRequest, ClientMessage as CM, Delete, Err, ErrorCode, Get, Key,
    KeyValuePair, KeyValuePairs, LiveOnlyFlag, Ls, LsState, MetaData, PDelete, PGet, PState,
    PStateEvent, PSubscribe, Privilege, Protocol, ProtocolVersion, Publish, RegularKeySegment,
    RequestPattern, ServerMessage, Set, State, StateEvent, Subscribe, SubscribeLs, TransactionId,
    UniqueFlag, Unsubscribe, UnsubscribeLs, Value,
};

async fn check_auth(
    auth_required: bool,
    privilege: Privilege,
    pattern: &str,
    auth: &Option<JwtClaims>,
    client: &mpsc::Sender<ServerMessage>,
    transaction_id: u64,
) -> WorterbuchResult<()> {
    if auth_required {
        match auth {
            Some(claims) => {
                if let Err(e) = claims.authorize(&privilege, pattern) {
                    handle_store_error(
                        WorterbuchError::Unauthorized(e.clone()),
                        client,
                        transaction_id,
                    )
                    .await?;
                    return Err(WorterbuchError::Unauthorized(e));
                }
            }
            None => return Err(WorterbuchError::AuthenticationRequired(privilege)),
        }
    }
    Ok(())
}

pub async fn process_incoming_message(
    client_id: Uuid,
    msg: &str,
    worterbuch: &CloneableWbApi,
    tx: &mpsc::Sender<ServerMessage>,
    auth_required: bool,
    auth: Option<JwtClaims>,
    config: &Config,
) -> WorterbuchResult<(bool, Option<JwtClaims>)> {
    log::debug!("Received message: {msg}");
    let mut authenticated = None;
    match serde_json::from_str(msg) {
        Ok(Some(msg)) => match msg {
            CM::AuthenticationRequest(msg) => {
                if auth.is_some() {
                    return Err(WorterbuchError::AlreadyAuthenticated);
                }
                authenticated = Some(authenticate(msg, tx, &config).await?);
            }
            CM::Get(msg) => {
                check_auth(
                    auth_required,
                    Privilege::Read,
                    &msg.key,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                get(msg, worterbuch, tx).await?;
            }
            CM::PGet(msg) => {
                check_auth(
                    auth_required,
                    Privilege::Read,
                    &msg.request_pattern,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                pget(msg, worterbuch, tx).await?;
            }
            CM::Set(msg) => {
                check_auth(
                    auth_required,
                    Privilege::Write,
                    &msg.key,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                set(msg, worterbuch, tx, client_id.to_string()).await?;
            }
            CM::Publish(msg) => {
                check_auth(
                    auth_required,
                    Privilege::Write,
                    &msg.key,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                publish(msg, worterbuch, tx).await?;
            }
            CM::Subscribe(msg) => {
                check_auth(
                    auth_required,
                    Privilege::Read,
                    &msg.key,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                subscribe(msg, client_id, worterbuch, tx).await?;
            }
            CM::PSubscribe(msg) => {
                check_auth(
                    auth_required,
                    Privilege::Read,
                    &msg.request_pattern,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                psubscribe(msg, client_id, worterbuch, tx).await?;
            }
            CM::Unsubscribe(msg) => unsubscribe(msg, worterbuch, tx, client_id).await?,
            CM::Delete(msg) => {
                check_auth(
                    auth_required,
                    Privilege::Delete,
                    &msg.key,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                delete(msg, worterbuch, tx, client_id.to_string()).await?;
            }
            CM::PDelete(msg) => {
                check_auth(
                    auth_required,
                    Privilege::Delete,
                    &msg.request_pattern,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                pdelete(msg, worterbuch, tx, client_id.to_string()).await?;
            }
            CM::Ls(msg) => {
                let pattern = &msg
                    .parent
                    .as_ref()
                    .map(|it| format!("{it}/?"))
                    .unwrap_or("?".to_owned());
                check_auth(
                    auth_required,
                    Privilege::Read,
                    pattern,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                ls(msg, worterbuch, tx).await?;
            }
            CM::SubscribeLs(msg) => {
                let pattern = &msg
                    .parent
                    .as_ref()
                    .map(|it| format!("{it}/?"))
                    .unwrap_or("?".to_owned());
                check_auth(
                    auth_required,
                    Privilege::Read,
                    pattern,
                    &auth,
                    tx,
                    msg.transaction_id,
                )
                .await?;
                subscribe_ls(msg, client_id, worterbuch, tx).await?;
            }
            CM::UnsubscribeLs(msg) => {
                unsubscribe_ls(msg, client_id, worterbuch, tx).await?;
            }
            CM::Keepalive => (),
        },
        Ok(None) => {
            // client disconnected
            return Ok((false, authenticated));
        }
        Err(e) => {
            log::error!("Error decoding message: {e}");
            return Ok((false, authenticated));
        }
    }

    Ok((true, authenticated))
}

pub enum WbFunction {
    Get(Key, oneshot::Sender<WorterbuchResult<(String, Value)>>),
    Set(Key, Value, String, oneshot::Sender<WorterbuchResult<()>>),
    Publish(Key, Value, oneshot::Sender<WorterbuchResult<()>>),
    Ls(
        Option<Key>,
        oneshot::Sender<WorterbuchResult<Vec<RegularKeySegment>>>,
    ),
    PGet(
        RequestPattern,
        oneshot::Sender<WorterbuchResult<KeyValuePairs>>,
    ),
    Subscribe(
        Uuid,
        TransactionId,
        Key,
        UniqueFlag,
        LiveOnlyFlag,
        oneshot::Sender<WorterbuchResult<(UnboundedReceiver<PStateEvent>, SubscriptionId)>>,
    ),
    PSubscribe(
        Uuid,
        TransactionId,
        RequestPattern,
        UniqueFlag,
        LiveOnlyFlag,
        oneshot::Sender<WorterbuchResult<(UnboundedReceiver<PStateEvent>, SubscriptionId)>>,
    ),
    SubscribeLs(
        Uuid,
        TransactionId,
        Option<Key>,
        oneshot::Sender<
            WorterbuchResult<(UnboundedReceiver<Vec<RegularKeySegment>>, SubscriptionId)>,
        >,
    ),
    Unsubscribe(Uuid, TransactionId, oneshot::Sender<WorterbuchResult<()>>),
    UnsubscribeLs(Uuid, TransactionId, oneshot::Sender<WorterbuchResult<()>>),
    Delete(Key, String, oneshot::Sender<WorterbuchResult<(Key, Value)>>),
    PDelete(
        RequestPattern,
        String,
        oneshot::Sender<WorterbuchResult<KeyValuePairs>>,
    ),
    Connected(Uuid, SocketAddr, Protocol),
    Disconnected(Uuid, SocketAddr),
    Config(oneshot::Sender<Config>),
    Export(oneshot::Sender<WorterbuchResult<Value>>),
    Len(oneshot::Sender<usize>),
    SupportedProtocolVersion(oneshot::Sender<ProtocolVersion>),
}

#[derive(Clone)]
pub struct CloneableWbApi {
    tx: mpsc::Sender<WbFunction>,
}

impl CloneableWbApi {
    pub fn new(tx: mpsc::Sender<WbFunction>) -> Self {
        CloneableWbApi { tx }
    }

    pub async fn get(&self, key: Key) -> WorterbuchResult<(String, Value)> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Get(key, tx)).await?;
        rx.await?
    }

    pub async fn pget<'a>(&self, pattern: RequestPattern) -> WorterbuchResult<KeyValuePairs> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::PGet(pattern, tx)).await?;
        rx.await?
    }

    pub async fn set(&self, key: Key, value: Value, client_id: String) -> WorterbuchResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::Set(key, value, client_id, tx))
            .await?;
        rx.await?
    }

    pub async fn publish(&self, key: Key, value: Value) -> WorterbuchResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Publish(key, value, tx)).await?;
        rx.await?
    }

    pub async fn ls(&self, parent: Option<Key>) -> WorterbuchResult<Vec<RegularKeySegment>> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Ls(parent, tx)).await?;
        rx.await?
    }

    pub async fn subscribe(
        &self,
        client_id: Uuid,
        transaction_id: TransactionId,
        key: Key,
        unique: bool,
        live_only: bool,
    ) -> WorterbuchResult<(UnboundedReceiver<PStateEvent>, SubscriptionId)> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::Subscribe(
                client_id,
                transaction_id,
                key,
                unique,
                live_only,
                tx,
            ))
            .await?;
        rx.await?
    }

    pub async fn psubscribe(
        &self,
        client_id: Uuid,
        transaction_id: TransactionId,
        pattern: RequestPattern,
        unique: bool,
        live_only: bool,
    ) -> WorterbuchResult<(UnboundedReceiver<PStateEvent>, SubscriptionId)> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::PSubscribe(
                client_id,
                transaction_id,
                pattern,
                unique,
                live_only,
                tx,
            ))
            .await?;
        rx.await?
    }

    pub async fn subscribe_ls(
        &self,
        client_id: Uuid,
        transaction_id: TransactionId,
        parent: Option<Key>,
    ) -> WorterbuchResult<(UnboundedReceiver<Vec<RegularKeySegment>>, SubscriptionId)> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::SubscribeLs(
                client_id,
                transaction_id,
                parent,
                tx,
            ))
            .await?;
        rx.await?
    }

    pub async fn unsubscribe(
        &self,
        client_id: Uuid,
        transaction_id: TransactionId,
    ) -> WorterbuchResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::Unsubscribe(client_id, transaction_id, tx))
            .await?;
        rx.await?
    }

    pub async fn unsubscribe_ls(
        &self,
        client_id: Uuid,
        transaction_id: TransactionId,
    ) -> WorterbuchResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::UnsubscribeLs(client_id, transaction_id, tx))
            .await?;
        rx.await?
    }

    pub async fn delete(&self, key: Key, client_id: String) -> WorterbuchResult<(Key, Value)> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Delete(key, client_id, tx)).await?;
        rx.await?
    }

    pub async fn pdelete(
        &self,
        pattern: RequestPattern,
        client_id: String,
    ) -> WorterbuchResult<KeyValuePairs> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::PDelete(pattern, client_id, tx))
            .await?;
        rx.await?
    }

    pub async fn connected(
        &self,
        client_id: Uuid,
        remote_addr: SocketAddr,
        protocol: Protocol,
    ) -> WorterbuchResult<()> {
        self.tx
            .send(WbFunction::Connected(client_id, remote_addr, protocol))
            .await?;
        Ok(())
    }

    pub async fn disconnected(
        &self,
        client_id: Uuid,
        remote_addr: SocketAddr,
    ) -> WorterbuchResult<()> {
        self.tx
            .send(WbFunction::Disconnected(client_id, remote_addr))
            .await?;
        Ok(())
    }

    pub async fn config(&self) -> WorterbuchResult<Config> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Config(tx)).await?;
        Ok(rx.await?)
    }

    pub async fn export(&self) -> WorterbuchResult<Value> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Export(tx)).await?;
        rx.await?
    }

    pub async fn len(&self) -> WorterbuchResult<usize> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Len(tx)).await?;
        Ok(rx.await?)
    }

    pub async fn supported_protocol_version(&self) -> WorterbuchResult<ProtocolVersion> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::SupportedProtocolVersion(tx))
            .await?;
        Ok(rx.await?)
    }
}

async fn authenticate(
    msg: AuthenticationRequest,
    client: &mpsc::Sender<ServerMessage>,
    config: &Config,
) -> WorterbuchResult<JwtClaims> {
    match get_claims(Some(&msg.auth_token), config) {
        Ok(claims) => {
            client
                .send(ServerMessage::Authenticated(Ack { transaction_id: 0 }))
                .await
                .context(|| "Error sending HANDSHAKE message".to_owned())?;
            Ok(claims)
        }
        Err(e) => {
            handle_store_error(WorterbuchError::Unauthorized(e.clone()), client, 0).await?;
            return Err(WorterbuchError::Unauthorized(e));
        }
    }
}

async fn get(
    msg: Get,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
) -> WorterbuchResult<()> {
    let key_value = match worterbuch.get(msg.key).await {
        Ok(key_value) => key_value.into(),
        Err(e) => {
            handle_store_error(e, client, msg.transaction_id).await?;
            return Ok(());
        }
    };

    let response = State {
        transaction_id: msg.transaction_id,
        event: StateEvent::KeyValue(key_value),
    };

    client
        .send(ServerMessage::State(response))
        .await
        .context(|| {
            format!(
                "Error sending STATE message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn pget(
    msg: PGet,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
) -> WorterbuchResult<()> {
    let values = match worterbuch.pget(msg.request_pattern.clone()).await {
        Ok(values) => values.into_iter().map(KeyValuePair::from).collect(),
        Err(e) => {
            handle_store_error(e, client, msg.transaction_id).await?;
            return Ok(());
        }
    };

    let response = PState {
        transaction_id: msg.transaction_id,
        request_pattern: msg.request_pattern,
        event: PStateEvent::KeyValuePairs(values),
    };

    client
        .send(ServerMessage::PState(response))
        .await
        .context(|| {
            format!(
                "Error sending PSTATE message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn set(
    msg: Set,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
    client_id: String,
) -> WorterbuchResult<()> {
    if let Err(e) = worterbuch.set(msg.key, msg.value, client_id).await {
        handle_store_error(e, client, msg.transaction_id).await?;
        return Ok(());
    }

    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    client
        .send(ServerMessage::Ack(response))
        .await
        .context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn publish(
    msg: Publish,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
) -> WorterbuchResult<()> {
    if let Err(e) = worterbuch.publish(msg.key, msg.value).await {
        handle_store_error(e, client, msg.transaction_id).await?;
        return Ok(());
    }

    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    client
        .send(ServerMessage::Ack(response))
        .await
        .context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn subscribe(
    msg: Subscribe,
    client_id: Uuid,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
) -> WorterbuchResult<bool> {
    let (mut rx, subscription) = match worterbuch
        .subscribe(
            client_id,
            msg.transaction_id,
            msg.key.clone(),
            msg.unique,
            msg.live_only.unwrap_or(false),
        )
        .await
    {
        Ok(it) => it,
        Err(e) => {
            handle_store_error(e, client, msg.transaction_id).await?;
            return Ok(false);
        }
    };

    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    client
        .send(ServerMessage::Ack(response))
        .await
        .context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    let transaction_id = msg.transaction_id;

    let wb_unsub = worterbuch.clone();
    let client_sub = client.clone();

    spawn(async move {
        log::debug!("Receiving events for subscription {subscription:?} …");
        while let Some(event) = rx.recv().await {
            let state_events: Vec<StateEvent> = event.into();

            for event in state_events {
                let state = State {
                    transaction_id,
                    event,
                };
                if let Err(e) = client_sub.send(ServerMessage::State(state)).await {
                    log::error!("Error sending STATE message to client: {e}");
                    break;
                };
            }
        }

        match wb_unsub.unsubscribe(client_id, transaction_id).await {
            Ok(()) => {
                log::warn!("Subscription was not cleaned up properly!");
            }
            Err(WorterbuchError::NotSubscribed) => { /* this is expected */ }
            Err(e) => {
                log::warn!("Error while unsubscribing: {e}");
            }
        }
    });

    Ok(true)
}

async fn psubscribe(
    msg: PSubscribe,
    client_id: Uuid,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
) -> WorterbuchResult<bool> {
    let live_only = msg.live_only.unwrap_or(false);

    let (rx, subscription) = match worterbuch
        .psubscribe(
            client_id,
            msg.transaction_id,
            msg.request_pattern.clone(),
            msg.unique,
            live_only,
        )
        .await
    {
        Ok(rx) => rx,
        Err(e) => {
            handle_store_error(e, client, msg.transaction_id).await?;
            return Ok(false);
        }
    };

    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    client
        .send(ServerMessage::Ack(response))
        .await
        .context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    let transaction_id = msg.transaction_id;
    let request_pattern = msg.request_pattern;

    let wb_unsub = worterbuch.clone();
    let client_sub = client.clone();

    let aggregate_events = msg.aggregate_events.map(Duration::from_millis);
    if let Some(aggregate_duration) = aggregate_events {
        spawn(async move {
            aggregate_loop(
                rx,
                transaction_id,
                request_pattern,
                client_sub,
                subscription,
                aggregate_duration,
                live_only,
            )
            .await;

            match wb_unsub.unsubscribe(client_id, transaction_id).await {
                Ok(()) => {
                    log::warn!("Subscription was not cleaned up properly!");
                }
                Err(WorterbuchError::NotSubscribed) => { /* this is expected */ }
                Err(e) => {
                    log::warn!("Error while unsubscribing: {e}");
                }
            }
        });
    } else {
        spawn(async move {
            forward_loop(
                rx,
                transaction_id,
                request_pattern,
                client_sub,
                subscription,
            )
            .await;

            match wb_unsub.unsubscribe(client_id, transaction_id).await {
                Ok(()) => {
                    log::warn!("Subscription was not cleaned up properly!");
                }
                Err(WorterbuchError::NotSubscribed) => { /* this is expected */ }
                Err(e) => {
                    log::warn!("Error while unsubscribing: {e}");
                }
            }
        });
    }

    Ok(true)
}

async fn forward_loop(
    mut rx: UnboundedReceiver<PStateEvent>,
    transaction_id: u64,
    request_pattern: String,
    client_sub: mpsc::Sender<ServerMessage>,
    subscription: SubscriptionId,
) {
    log::debug!("Receiving events for subscription {subscription:?} …");
    while let Some(event) = rx.recv().await {
        let event = PState {
            transaction_id,
            request_pattern: request_pattern.clone(),
            event,
        };
        if let Err(e) = client_sub.send(ServerMessage::PState(event)).await {
            log::error!("Error sending STATE message to client: {e}");
            break;
        }
    }
}

async fn aggregate_loop(
    mut rx: UnboundedReceiver<PStateEvent>,
    transaction_id: u64,
    request_pattern: String,
    client_sub: mpsc::Sender<ServerMessage>,
    subscription: SubscriptionId,
    aggregate_duration: Duration,
    live_only: bool,
) {
    if !live_only {
        log::debug!("Immediately forwarding current state to new subscription {subscription:?} …");

        if let Some(event) = rx.recv().await {
            let event = PState {
                transaction_id,
                request_pattern: request_pattern.clone(),
                event,
            };

            if let Err(e) = client_sub.send(ServerMessage::PState(event)).await {
                log::error!("Error sending STATE message to client: {e}");
                return;
            }
        } else {
            return;
        }
    }

    log::debug!("Aggregating events for subscription {subscription:?} …");

    let aggregator = PStateAggregator::new(
        client_sub,
        request_pattern,
        aggregate_duration,
        transaction_id,
    );

    while let Some(event) = rx.recv().await {
        if let Err(e) = aggregator.aggregate(event) {
            log::error!("Error sending STATE message to client: {e}");
            break;
        }
    }
}

async fn unsubscribe(
    msg: Unsubscribe,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
    client_id: Uuid,
) -> WorterbuchResult<()> {
    if let Err(e) = worterbuch.unsubscribe(client_id, msg.transaction_id).await {
        handle_store_error(e, client, msg.transaction_id).await?;
        return Ok(());
    };
    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    client
        .send(ServerMessage::Ack(response))
        .await
        .context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn delete(
    msg: Delete,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
    client_id: String,
) -> WorterbuchResult<()> {
    let key_value = match worterbuch.delete(msg.key, client_id).await {
        Ok(key_value) => key_value.into(),
        Err(e) => {
            handle_store_error(e, client, msg.transaction_id).await?;
            return Ok(());
        }
    };

    let response = State {
        transaction_id: msg.transaction_id,
        event: StateEvent::Deleted(key_value),
    };

    client
        .send(ServerMessage::State(response))
        .await
        .context(|| {
            format!(
                "Error sending STATE message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn pdelete(
    msg: PDelete,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
    client_id: String,
) -> WorterbuchResult<()> {
    let deleted = match worterbuch
        .pdelete(msg.request_pattern.clone(), client_id)
        .await
    {
        Ok(it) => it,
        Result::Err(e) => {
            handle_store_error(e, client, msg.transaction_id).await?;
            return Ok(());
        }
    };

    let response = PState {
        transaction_id: msg.transaction_id,
        request_pattern: msg.request_pattern,
        event: PStateEvent::Deleted(deleted),
    };

    client
        .send(ServerMessage::PState(response))
        .await
        .context(|| {
            format!(
                "Error sending PSTATE message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn ls(
    msg: Ls,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
) -> WorterbuchResult<()> {
    let children = match worterbuch.ls(msg.parent).await {
        Ok(it) => it,
        Result::Err(e) => {
            handle_store_error(e, client, msg.transaction_id).await?;
            return Ok(());
        }
    };

    let response = LsState {
        transaction_id: msg.transaction_id,
        children,
    };

    client
        .send(ServerMessage::LsState(response))
        .await
        .context(|| {
            format!(
                "Error sending LSSTATE message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn subscribe_ls(
    msg: SubscribeLs,
    client_id: Uuid,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
) -> WorterbuchResult<bool> {
    let (mut rx, subscription) = match worterbuch
        .subscribe_ls(client_id, msg.transaction_id, msg.parent.clone())
        .await
    {
        Ok(it) => it,
        Err(e) => {
            handle_store_error(e, client, msg.transaction_id).await?;
            return Ok(false);
        }
    };

    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    client
        .send(ServerMessage::Ack(response))
        .await
        .context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    let transaction_id = msg.transaction_id;

    let wb_unsub = worterbuch.clone();
    let client_sub = client.clone();

    spawn(async move {
        log::debug!("Receiving events for ls subscription {subscription:?} …");
        while let Some(children) = rx.recv().await {
            let state = LsState {
                transaction_id,
                children,
            };
            if let Err(e) = client_sub.send(ServerMessage::LsState(state)).await {
                log::error!("Error sending STATE message to client: {e}");
                break;
            };
        }

        match wb_unsub.unsubscribe_ls(client_id, transaction_id).await {
            Ok(()) => {
                log::warn!("Ls Subscription was not cleaned up properly!");
            }
            Err(WorterbuchError::NotSubscribed) => { /* this is expected */ }
            Err(e) => {
                log::warn!("Error while unsubscribing ls: {e}");
            }
        }
    });

    Ok(true)
}

async fn unsubscribe_ls(
    msg: UnsubscribeLs,
    client_id: Uuid,
    worterbuch: &CloneableWbApi,
    client: &mpsc::Sender<ServerMessage>,
) -> WorterbuchResult<()> {
    if let Err(e) = worterbuch
        .unsubscribe_ls(client_id, msg.transaction_id)
        .await
    {
        handle_store_error(e, client, msg.transaction_id).await?;
        return Ok(());
    }
    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    client
        .send(ServerMessage::Ack(response))
        .await
        .context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?;

    Ok(())
}

async fn handle_store_error(
    e: WorterbuchError,
    client: &mpsc::Sender<ServerMessage>,
    transaction_id: u64,
) -> WorterbuchResult<()> {
    let error_code = ErrorCode::from(&e);
    let err_msg = match e {
        WorterbuchError::IllegalWildcard(pattern) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(&pattern).expect("failed to serialize metadata"),
        },
        WorterbuchError::IllegalMultiWildcard(pattern) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(&pattern).expect("failed to serialize metadata"),
        },
        WorterbuchError::MultiWildcardAtIllegalPosition(pattern) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(&pattern).expect("failed to serialize metadata"),
        },
        WorterbuchError::NoSuchValue(key) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(&format!("no value for key '{key}'"))
                .expect("failed to serialize error message"),
        },
        WorterbuchError::NotSubscribed => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(&format!(
                "no subscription found for transaction id '{transaction_id}'"
            ))
            .expect("failed to serialize error message"),
        },
        WorterbuchError::IoError(e, meta) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string::<Meta>(&(&e.into(), meta).into())
                .expect("failed to serialize metadata"),
        },
        WorterbuchError::SerDeError(e, meta) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string::<Meta>(&(&e.into(), meta).into())
                .expect("failed to serialize metadata"),
        },
        WorterbuchError::ProtocolNegotiationFailed => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(
                "server does not implement any of the protocl versions supported by this client",
            )
            .expect("failed to serialize metadata"),
        },
        WorterbuchError::Other(e, meta) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string::<Meta>(&(&e, meta).into())
                .expect("failed to serialize metadata"),
        },
        WorterbuchError::ServerResponse(_) | WorterbuchError::InvalidServerResponse(_) => {
            panic!("store must not produce this error")
        }
        WorterbuchError::ReadOnlyKey(key) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(&format!("tried to delete read only key '{key}'"))
                .expect("failed to serialize error message"),
        },
        WorterbuchError::AuthenticationFailed => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string("client failed to authenticate")
                .expect("failed to serialize error message"),
        },
        WorterbuchError::AuthenticationRequired(op) => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(&format!("operation {op} requires authentication"))
                .expect("failed to serialize error message"),
        },
        WorterbuchError::AlreadyAuthenticated => Err {
            error_code,
            transaction_id,
            metadata: serde_json::to_string(
                "handshake has already been completed, cannot do it again",
            )
            .expect("failed to serialize error message"),
        },
        WorterbuchError::Unauthorized(auth_err) => Err {
            error_code,
            transaction_id,
            metadata: auth_err.to_string(),
        },
    };
    client
        .send(ServerMessage::Err(err_msg))
        .await
        .context(|| "Error sending ERR message to client".to_owned())
}

#[derive(Serialize)]
struct Meta {
    cause: String,
    meta: MetaData,
}

impl From<(&Box<dyn std::error::Error + Send + Sync>, MetaData)> for Meta {
    fn from(e: (&Box<dyn std::error::Error + Send + Sync>, MetaData)) -> Self {
        Meta {
            cause: e.0.to_string(),
            meta: e.1,
        }
    }
}
