mod v1_0;

use crate::{subscribers::SubscriptionId, Config};
use std::net::SocketAddr;
use tokio::sync::{
    mpsc::{self, UnboundedReceiver},
    oneshot,
};
use uuid::Uuid;
use worterbuch_common::{
    error::WorterbuchResult, Handshake, Key, KeyValuePairs, PStateEvent, ProtocolVersion,
    ProtocolVersions, RegularKeySegment, RequestPattern, ServerMessage, TransactionId, Value,
};

pub async fn process_incoming_message(
    client_id: Uuid,
    msg: &str,
    worterbuch: &CloneableWbApi,
    tx: &mpsc::Sender<ServerMessage>,
    protocol_version: &ProtocolVersion,
) -> WorterbuchResult<(bool, bool)> {
    match protocol_version {
        ProtocolVersion { major, minor } if *major < 1 || (*major == 1 && *minor == 0) => {
            v1_0::process_incoming_message(client_id, msg, worterbuch, tx).await
        }
        _ => panic!("looks like the server accidentally accepted a connection to a client that speaks an unsupported protocol version"),
    }
}

pub enum WbFunction {
    Handshake(
        ProtocolVersions,
        KeyValuePairs,
        Vec<String>,
        Uuid,
        oneshot::Sender<WorterbuchResult<Handshake>>,
    ),
    Get(Key, oneshot::Sender<WorterbuchResult<(String, Value)>>),
    Set(Key, Value, oneshot::Sender<WorterbuchResult<()>>),
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
        bool,
        oneshot::Sender<WorterbuchResult<(UnboundedReceiver<PStateEvent>, SubscriptionId)>>,
    ),
    PSubscribe(
        Uuid,
        TransactionId,
        RequestPattern,
        bool,
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
    Delete(Key, oneshot::Sender<WorterbuchResult<(Key, Value)>>),
    PDelete(
        RequestPattern,
        oneshot::Sender<WorterbuchResult<KeyValuePairs>>,
    ),
    Connected(Uuid, SocketAddr),
    Disconnected(Uuid, SocketAddr),
    Config(oneshot::Sender<Config>),
    Export(oneshot::Sender<WorterbuchResult<Value>>),
    Len(oneshot::Sender<usize>),
    SupportedProtocolVersions(oneshot::Sender<ProtocolVersions>),
}

#[derive(Clone)]
pub struct CloneableWbApi {
    tx: mpsc::Sender<WbFunction>,
}

impl CloneableWbApi {
    pub fn new(tx: mpsc::Sender<WbFunction>) -> Self {
        CloneableWbApi { tx }
    }

    pub async fn handshake(
        &self,
        supported_protocol_versions: ProtocolVersions,
        last_will: KeyValuePairs,
        grave_goods: Vec<String>,
        client_id: Uuid,
    ) -> WorterbuchResult<Handshake> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::Handshake(
                supported_protocol_versions,
                last_will,
                grave_goods,
                client_id,
                tx,
            ))
            .await?;
        rx.await?
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

    pub async fn set(&self, key: Key, value: Value) -> WorterbuchResult<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Set(key, value, tx)).await?;
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
    ) -> WorterbuchResult<(UnboundedReceiver<PStateEvent>, SubscriptionId)> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::Subscribe(
                client_id,
                transaction_id,
                key,
                unique,
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
    ) -> WorterbuchResult<(UnboundedReceiver<PStateEvent>, SubscriptionId)> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::PSubscribe(
                client_id,
                transaction_id,
                pattern,
                unique,
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

    pub async fn delete(&self, key: Key) -> WorterbuchResult<(Key, Value)> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::Delete(key, tx)).await?;
        rx.await?
    }

    pub async fn pdelete(&self, pattern: RequestPattern) -> WorterbuchResult<KeyValuePairs> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(WbFunction::PDelete(pattern, tx)).await?;
        rx.await?
    }

    pub async fn connected(
        &self,
        client_id: Uuid,
        remote_addr: SocketAddr,
    ) -> WorterbuchResult<()> {
        self.tx
            .send(WbFunction::Connected(client_id, remote_addr))
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

    pub async fn supported_protocol_versions(&self) -> WorterbuchResult<ProtocolVersions> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WbFunction::SupportedProtocolVersions(tx))
            .await?;
        Ok(rx.await?)
    }
}
