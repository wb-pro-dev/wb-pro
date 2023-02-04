use crate::{
    ErrorCode, Key, KeyValuePair, KeyValuePairs, Keys, MetaData, MultiWildcard, ProtocolVersion,
    RequestPattern, Separator, TransactionId, TypedKeyValuePair, Value, Wildcard,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;

pub const ILLEGAL_WILDCARD: ErrorCode = 0b00000000;
pub const ILLEGAL_MULTI_WILDCARD: ErrorCode = 0b00000001;
pub const MULTI_WILDCARD_AT_ILLEGAL_POSITION: ErrorCode = 0b00000010;
pub const IO_ERROR: ErrorCode = 0b00000011;
pub const SERDE_ERROR: ErrorCode = 0b00000100;
pub const NO_SUCH_VALUE: ErrorCode = 0b00000101;
pub const NOT_SUBSCRIBED: ErrorCode = 0b00000110;
pub const PROTOCOL_NEGOTIATION_FAILED: ErrorCode = 0b00000111;
pub const INVALID_SERVER_RESPONSE: ErrorCode = 0b00001000;
pub const OTHER: ErrorCode = 0b11111111;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerMessage {
    PState(PState),
    Ack(Ack),
    State(State),
    Err(Err),
    Handshake(Handshake),
}

impl ServerMessage {
    pub fn transaction_id(&self) -> u64 {
        match self {
            ServerMessage::PState(msg) => msg.transaction_id,
            ServerMessage::Ack(msg) => msg.transaction_id,
            ServerMessage::State(msg) => msg.transaction_id,
            ServerMessage::Err(msg) => msg.transaction_id,
            ServerMessage::Handshake(_) => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PState {
    pub transaction_id: TransactionId,
    pub request_pattern: RequestPattern,
    #[serde(flatten)]
    pub event: PStateEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PStateEvent {
    KeyValuePairs(KeyValuePairs),
    Deleted(Keys),
}

impl From<PStateEvent> for Vec<StateEvent> {
    fn from(e: PStateEvent) -> Self {
        match e {
            PStateEvent::KeyValuePairs(kvps) => kvps.into_iter().map(StateEvent::from).collect(),
            PStateEvent::Deleted(keys) => keys.into_iter().map(StateEvent::from).collect(),
        }
    }
}

impl From<PState> for Vec<StateEvent> {
    fn from(pstate: PState) -> Self {
        pstate.event.into()
    }
}

impl From<PStateEvent> for Vec<Option<Value>> {
    fn from(e: PStateEvent) -> Self {
        match e {
            PStateEvent::KeyValuePairs(kvps) => kvps.into_iter().map(KeyValuePair::into).collect(),
            PStateEvent::Deleted(keys) => keys.into_iter().map(|_| Option::None).collect(),
        }
    }
}

impl From<PState> for Vec<Option<Value>> {
    fn from(pstate: PState) -> Self {
        pstate.event.into()
    }
}

impl fmt::Display for PState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.event {
            PStateEvent::KeyValuePairs(key_value_pairs) => {
                let kvps: Vec<String> = key_value_pairs
                    .iter()
                    .map(|&KeyValuePair { ref key, ref value }| format!("{key}={value}"))
                    .collect();
                let joined = kvps.join("\n");
                write!(f, "{joined}")
            }
            PStateEvent::Deleted(keys) => {
                let kvps: Vec<String> = keys.iter().map(|key| format!("{key} deleted")).collect();
                let joined = kvps.join("\n");
                write!(f, "{joined}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ack {
    pub transaction_id: TransactionId,
}

impl fmt::Display for Ack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ack {}", self.transaction_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub transaction_id: TransactionId,
    #[serde(flatten)]
    pub event: StateEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StateEvent {
    KeyValue(KeyValuePair),
    Deleted(Key),
}

impl From<KeyValuePair> for StateEvent {
    fn from(kvp: KeyValuePair) -> Self {
        StateEvent::KeyValue(kvp)
    }
}

impl From<Key> for StateEvent {
    fn from(key: Key) -> Self {
        StateEvent::Deleted(key)
    }
}

impl From<StateEvent> for Option<Value> {
    fn from(e: StateEvent) -> Self {
        match e {
            StateEvent::KeyValue(kv) => Some(kv.value),
            StateEvent::Deleted(_) => None,
        }
    }
}

impl From<State> for Option<Value> {
    fn from(state: State) -> Self {
        state.event.into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedStateEvent<T: DeserializeOwned> {
    KeyValue(TypedKeyValuePair<T>),
    Deleted(Key),
}

impl<T: DeserializeOwned> From<TypedStateEvent<T>> for Option<T> {
    fn from(e: TypedStateEvent<T>) -> Self {
        match e {
            TypedStateEvent::KeyValue(kvp) => Some(kvp.value),
            TypedStateEvent::Deleted(_) => None,
        }
    }
}

impl<T: DeserializeOwned> From<TypedKeyValuePair<T>> for TypedStateEvent<T> {
    fn from(kvp: TypedKeyValuePair<T>) -> Self {
        TypedStateEvent::KeyValue(kvp)
    }
}

impl<T: DeserializeOwned> From<Key> for TypedStateEvent<T> {
    fn from(key: Key) -> Self {
        TypedStateEvent::Deleted(key)
    }
}

impl<T: DeserializeOwned> TryFrom<KeyValuePair> for TypedStateEvent<T> {
    type Error = serde_json::Error;

    fn try_from(kvp: KeyValuePair) -> Result<Self, Self::Error> {
        let typed: TypedKeyValuePair<T> = kvp.try_into()?;
        Ok(typed.into())
    }
}

impl<T: DeserializeOwned> TryFrom<StateEvent> for TypedStateEvent<T> {
    type Error = serde_json::Error;

    fn try_from(e: StateEvent) -> Result<Self, Self::Error> {
        match e {
            StateEvent::KeyValue(kvp) => Ok(kvp.try_into()?),
            StateEvent::Deleted(key) => Ok(key.into()),
        }
    }
}

pub type TypedStateEvents<T> = Vec<TypedStateEvent<T>>;

impl<T: DeserializeOwned> TryFrom<PStateEvent> for TypedStateEvents<T> {
    type Error = serde_json::Error;

    fn try_from(event: PStateEvent) -> Result<Self, Self::Error> {
        let state_events: Vec<StateEvent> = event.into();
        let mut typed_events = TypedStateEvents::new();
        for event in state_events {
            typed_events.push(event.try_into()?);
        }
        Ok(typed_events)
    }
}

impl<T: DeserializeOwned> TryFrom<PState> for TypedStateEvents<T> {
    type Error = serde_json::Error;

    fn try_from(pstate: PState) -> Result<Self, Self::Error> {
        Ok(pstate.event.try_into()?)
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.event {
            StateEvent::KeyValue(KeyValuePair { key, value }) => write!(f, "{key}={value}"),
            StateEvent::Deleted(key) => write!(f, "{key} deleted"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Err {
    pub transaction_id: TransactionId,
    pub error_code: ErrorCode,
    pub metadata: MetaData,
}

impl fmt::Display for Err {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "server error {}: {}", self.error_code, self.metadata)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Handshake {
    pub protocol_version: ProtocolVersion,
    pub separator: Separator,
    pub wildcard: Wildcard,
    pub multi_wildcard: MultiWildcard,
}

impl fmt::Display for Handshake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "handshake: separator: '{}', wildcard: '{}', multi-wildcard: '{}', supported protocol versions: {}",
            self.separator, self.wildcard, self.multi_wildcard, format!("{}.{}",self.protocol_version.major,self.protocol_version.minor)
        )
    }
}

#[cfg(test)]
mod test {

    use serde_json::json;

    use super::*;

    #[test]
    fn state_is_serialized_correctly() {
        let state = State {
            transaction_id: 1,
            event: StateEvent::KeyValue(("$SYS/clients", json!(2)).into()),
        };

        let json = r#"{"transactionId":1,"keyValue":{"key":"$SYS/clients","value":2}}"#;

        assert_eq!(json, &serde_json::to_string(&state).unwrap());

        let state = State {
            transaction_id: 1,
            event: StateEvent::Deleted("$SYS/clients".to_owned()),
        };

        let json = r#"{"transactionId":1,"deleted":"$SYS/clients"}"#;

        assert_eq!(json, &serde_json::to_string(&state).unwrap());
    }

    #[test]
    fn state_is_deserialized_correctly() {
        let state = State {
            transaction_id: 1,
            event: StateEvent::KeyValue(("$SYS/clients", json!(2)).into()),
        };

        let json = r#"{"transactionId":1,"keyValue":{"key":"$SYS/clients","value":2}}"#;

        assert_eq!(state, serde_json::from_str(&json).unwrap());

        let state = State {
            transaction_id: 1,
            event: StateEvent::Deleted("$SYS/clients".to_owned()),
        };

        let json = r#"{"transactionId":1,"deleted":"$SYS/clients"}"#;

        assert_eq!(state, serde_json::from_str(&json).unwrap());
    }

    #[test]
    fn pstate_is_serialized_correctly() {
        let pstate = PState {
            transaction_id: 1,
            request_pattern: "$SYS/clients".to_owned(),
            event: PStateEvent::KeyValuePairs(vec![("$SYS/clients", json!(2)).into()]),
        };

        let json = r#"{"transactionId":1,"requestPattern":"$SYS/clients","keyValuePairs":[{"key":"$SYS/clients","value":2}]}"#;

        assert_eq!(json, &serde_json::to_string(&pstate).unwrap());

        let pstate = PState {
            transaction_id: 1,
            request_pattern: "$SYS/clients".to_owned(),
            event: PStateEvent::Deleted(vec!["$SYS/clients".to_owned()]),
        };

        let json =
            r#"{"transactionId":1,"requestPattern":"$SYS/clients","deleted":["$SYS/clients"]}"#;

        assert_eq!(json, &serde_json::to_string(&pstate).unwrap());
    }

    #[test]
    fn pstate_is_deserialized_correctly() {
        let pstate = PState {
            transaction_id: 1,
            request_pattern: "$SYS/clients".to_owned(),
            event: PStateEvent::KeyValuePairs(vec![("$SYS/clients", json!(2)).into()]),
        };

        let json = r#"{"transactionId":1,"requestPattern":"$SYS/clients","keyValuePairs":[{"key":"$SYS/clients","value":2}]}"#;

        assert_eq!(pstate, serde_json::from_str(&json).unwrap());

        let pstate = PState {
            transaction_id: 1,
            request_pattern: "$SYS/clients".to_owned(),
            event: PStateEvent::Deleted(vec!["$SYS/clients".to_owned()]),
        };

        let json =
            r#"{"transactionId":1,"requestPattern":"$SYS/clients","deleted":["$SYS/clients"]}"#;

        assert_eq!(pstate, serde_json::from_str(&json).unwrap());
    }
}
