use serde::Serialize;
use tokio::{spawn, sync::mpsc::UnboundedSender};
use uuid::Uuid;
use worterbuch_common::{
    error::WorterbuchResult,
    error::{Context, WorterbuchError},
    Ack, ClientMessage as CM, Delete, Err, ErrorCode, Get, HandshakeRequest, KeyValuePair, Ls,
    LsState, MetaData, PDelete, PGet, PState, PStateEvent, PSubscribe, Publish, ServerMessage, Set,
    State, StateEvent, Subscribe, SubscribeLs, Unsubscribe, UnsubscribeLs,
};

use super::CloneableWbApi;

pub async fn process_incoming_message(
    client_id: Uuid,
    msg: &str,
    worterbuch: &CloneableWbApi,
    tx: UnboundedSender<String>,
) -> WorterbuchResult<(bool, bool)> {
    let mut hs = false;
    match serde_json::from_str(msg) {
        Ok(Some(msg)) => match msg {
            CM::HandshakeRequest(msg) => {
                hs = true;
                handshake(msg, worterbuch, &tx, client_id.clone()).await?;
            }
            CM::Get(msg) => {
                get(msg, worterbuch, &tx).await?;
            }
            CM::PGet(msg) => {
                pget(msg, worterbuch, &tx).await?;
            }
            CM::Set(msg) => {
                set(msg, worterbuch, &tx).await?;
            }
            CM::Publish(msg) => {
                publish(msg, worterbuch, &tx).await?;
            }
            CM::Subscribe(msg) => {
                let unique = msg.unique;
                subscribe(msg, client_id, worterbuch, &tx, unique).await?;
            }
            CM::PSubscribe(msg) => {
                let unique = msg.unique;
                psubscribe(msg, client_id, worterbuch, &tx, unique).await?;
            }
            CM::Unsubscribe(msg) => unsubscribe(msg, worterbuch, &tx, client_id).await?,
            CM::Delete(msg) => {
                delete(msg, worterbuch, &tx).await?;
            }
            CM::PDelete(msg) => {
                pdelete(msg, worterbuch, &tx).await?;
            }
            CM::Ls(msg) => {
                ls(msg, worterbuch, &tx).await?;
            }
            CM::SubscribeLs(msg) => {
                subscribe_ls(msg, client_id, worterbuch, &tx).await?;
            }
            CM::UnsubscribeLs(msg) => {
                unsubscribe_ls(msg, client_id, worterbuch, &tx).await?;
            }
            CM::Keepalive => (),
        },
        Ok(None) => {
            // client disconnected
            return Ok((false, hs));
        }
        Err(e) => {
            log::error!("Error decoding message: {e}");
            return Ok((false, hs));
        }
    }

    Ok((true, hs))
}

async fn handshake(
    msg: HandshakeRequest,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
    client_id: Uuid,
) -> WorterbuchResult<()> {
    let response = match worterbuch
        .handshake(
            msg.supported_protocol_versions,
            msg.last_will,
            msg.grave_goods,
            client_id,
        )
        .await
    {
        Ok(handshake) => handshake,
        Err(e) => {
            handle_store_error(e, client, 0).await?;
            return Ok(());
        }
    };

    match serde_json::to_string(&ServerMessage::Handshake(response)) {
        Ok(data) => client
            .send(data)
            .context(|| format!("Error sending HANDSHAKE message",))?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn get(
    msg: Get,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
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

    match serde_json::to_string(&ServerMessage::State(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending STATE message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn pget(
    msg: PGet,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
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

    match serde_json::to_string(&ServerMessage::PState(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending PSTATE message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn set(
    msg: Set,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
) -> WorterbuchResult<()> {
    if let Err(e) = worterbuch.set(msg.key, msg.value).await {
        handle_store_error(e, client, msg.transaction_id).await?;
        return Ok(());
    }

    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    match serde_json::to_string(&ServerMessage::Ack(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn publish(
    msg: Publish,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
) -> WorterbuchResult<()> {
    if let Err(e) = worterbuch.publish(msg.key, msg.value).await {
        handle_store_error(e, client, msg.transaction_id).await?;
        return Ok(());
    }

    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    match serde_json::to_string(&ServerMessage::Ack(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn subscribe(
    msg: Subscribe,
    client_id: Uuid,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
    unique: bool,
) -> WorterbuchResult<bool> {
    let (mut rx, subscription) = match worterbuch
        .subscribe(client_id, msg.transaction_id, msg.key.clone(), unique)
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

    match serde_json::to_string(&ServerMessage::Ack(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    let transaction_id = msg.transaction_id;

    let wb_unsub = worterbuch.clone();
    let client_sub = client.clone();

    spawn(async move {
        log::debug!("Receiving events for subscription {subscription:?} …");
        while let Some(event) = rx.recv().await {
            let state_events: Vec<StateEvent> = event.into();

            for event in state_events {
                let state = State {
                    transaction_id: transaction_id.clone(),
                    event,
                };
                match serde_json::to_string(&ServerMessage::State(state)) {
                    Ok(data) => {
                        if let Err(e) = client_sub.send(data) {
                            log::error!("Error sending STATE message to client: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        if let Err(e) = handle_encode_error(e).await {
                            log::error!("Error sending ERROR message to client: {e}");
                            break;
                        }
                    }
                }
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
    client: &UnboundedSender<String>,
    unique: bool,
) -> WorterbuchResult<bool> {
    let (mut rx, subscription) = match worterbuch
        .psubscribe(
            client_id,
            msg.transaction_id,
            msg.request_pattern.clone(),
            unique,
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

    match serde_json::to_string(&ServerMessage::Ack(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    let transaction_id = msg.transaction_id;
    let request_pattern = msg.request_pattern;

    let wb_unsub = worterbuch.clone();
    let client_sub = client.clone();

    spawn(async move {
        log::debug!("Receiving events for subscription {subscription:?} …");
        while let Some(event) = rx.recv().await {
            let event = PState {
                transaction_id: transaction_id.clone(),
                request_pattern: request_pattern.clone(),
                event,
            };
            match serde_json::to_string(&ServerMessage::PState(event)) {
                Ok(data) => {
                    if let Err(e) = client_sub.send(data) {
                        log::error!("Error sending STATE message to client: {e}");
                        break;
                    }
                }
                Err(e) => {
                    if let Err(e) = handle_encode_error(e).await {
                        log::error!("Error sending ERROR message to client: {e}");
                        break;
                    }
                }
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

async fn unsubscribe(
    msg: Unsubscribe,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
    client_id: Uuid,
) -> WorterbuchResult<()> {
    if let Err(e) = worterbuch.unsubscribe(client_id, msg.transaction_id).await {
        handle_store_error(e, client, msg.transaction_id).await?;
        return Ok(());
    };
    let response = Ack {
        transaction_id: msg.transaction_id,
    };

    match serde_json::to_string(&ServerMessage::Ack(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn delete(
    msg: Delete,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
) -> WorterbuchResult<()> {
    let key_value = match worterbuch.delete(msg.key).await {
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

    match serde_json::to_string(&ServerMessage::State(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending STATE message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn pdelete(
    msg: PDelete,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
) -> WorterbuchResult<()> {
    let deleted = match worterbuch.pdelete(msg.request_pattern.clone()).await {
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

    match serde_json::to_string(&ServerMessage::PState(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending PSTATE message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn ls(
    msg: Ls,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
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

    match serde_json::to_string(&ServerMessage::LsState(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending LSSTATE message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn subscribe_ls(
    msg: SubscribeLs,
    client_id: Uuid,
    worterbuch: &CloneableWbApi,
    client: &UnboundedSender<String>,
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

    match serde_json::to_string(&ServerMessage::Ack(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    let transaction_id = msg.transaction_id;

    let wb_unsub = worterbuch.clone();
    let client_sub = client.clone();

    spawn(async move {
        log::debug!("Receiving events for ls subscription {subscription:?} …");
        while let Some(children) = rx.recv().await {
            let state = LsState {
                transaction_id: transaction_id.clone(),
                children,
            };
            match serde_json::to_string(&ServerMessage::LsState(state)) {
                Ok(data) => {
                    if let Err(e) = client_sub.send(data) {
                        log::error!("Error sending STATE message to client: {e}");
                        break;
                    }
                }
                Err(e) => {
                    if let Err(e) = handle_encode_error(e).await {
                        log::error!("Error sending ERROR message to client: {e}");
                        break;
                    }
                }
            }
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
    client: &UnboundedSender<String>,
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

    match serde_json::to_string(&ServerMessage::Ack(response)) {
        Ok(data) => client.send(data).context(|| {
            format!(
                "Error sending ACK message for transaction ID {}",
                msg.transaction_id
            )
        })?,
        Err(e) => handle_encode_error(e).await?,
    }

    Ok(())
}

async fn handle_encode_error(e: serde_json::Error) -> WorterbuchResult<()> {
    panic!("Failed to encode a value to JSON: {e}");
}

async fn handle_store_error(
    e: WorterbuchError,
    client: &UnboundedSender<String>,
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
        WorterbuchError::SerDeYamlError(e, meta) => Err {
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
    };
    let msg = serde_json::to_string(&ServerMessage::Err(err_msg))
        .expect(&format!("failed to encode error message"));
    client
        .send(msg)
        .context(|| format!("Error sending ERR message to client"))
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
