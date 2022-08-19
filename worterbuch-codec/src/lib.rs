pub mod blocking;
pub mod error;
mod nonblocking;

use crate::error::{EncodeError, EncodeResult};
pub use nonblocking::*;
use serde::{Deserialize, Serialize};
use std::fmt;

pub type MessageType = u8;
pub type TransactionId = u64;
pub type RequestPattern = String;
pub type Key = String;
pub type Value = String;
pub type KeyValuePairs = Vec<KeyValuePair>;
pub type ErrorCode = u8;
pub type MetaData = String;
pub type PathLength = u16;
pub type Path = String;
pub type Separator = char;
pub type Wildcard = char;
pub type MultiWildcard = char;
pub type ProtocolVersionSegment = u16;
pub type ProtocolVersions = Vec<ProtocolVersion>;

pub type RequestPatternLength = u16;
pub type KeyLength = u16;
pub type ValueLength = u32;
pub type MetaDataLength = u32;
pub type NumKeyValuePairs = u32;
pub type NumProtocolVersions = u8;

pub const GET: MessageType = 0b00000000;
pub const SET: MessageType = 0b00000001;
pub const SUB: MessageType = 0b00000010;
pub const PGET: MessageType = 0b00000011;
pub const PSUB: MessageType = 0b00000100;
pub const EXP: MessageType = 0b00000101;
pub const IMP: MessageType = 0b00000110;
pub const USUB: MessageType = 0b00000111;

pub const PSTA: MessageType = 0b10000000;
pub const ACK: MessageType = 0b10000001;
pub const STA: MessageType = 0b10000010;
pub const ERR: MessageType = 0b10000011;
pub const HSHK: MessageType = 0b10000100;

pub const ILLEGAL_WILDCARD: ErrorCode = 0b00000000;
pub const ILLEGAL_MULTI_WILDCARD: ErrorCode = 0b00000001;
pub const MULTI_WILDCARD_AT_ILLEGAL_POSITION: ErrorCode = 0b00000010;
pub const IO_ERROR: ErrorCode = 0b00000011;
pub const SERDE_ERROR: ErrorCode = 0b00000100;
pub const NO_SUCH_VALUE: ErrorCode = 0b00000101;
pub const NOT_SUBSCRIBED: ErrorCode = 0b00000110;
pub const OTHER: ErrorCode = 0b11111111;

pub const TRANSACTION_ID_BYTES: usize = 8;
pub const REQUEST_PATTERN_LENGTH_BYTES: usize = 2;
pub const KEY_LENGTH_BYTES: usize = 2;
pub const VALUE_LENGTH_BYTES: usize = 4;
pub const NUM_KEY_VALUE_PAIRS_BYTES: usize = 4;
pub const ERROR_CODE_BYTES: usize = 1;
pub const METADATA_LENGTH_BYTES: usize = 4;
pub const PATH_LENGTH_BYTES: usize = 2;
pub const UNIQUE_FLAG_BYTES: usize = 1;
pub const PROTOCOL_VERSION_SEGMENT_BYTES: usize = 2;
pub const NUM_PROTOCOL_VERSION_BYTES: usize = 1;
pub const SEPARATOR_BYTES: usize = 1;
pub const WILDCARD_BYTES: usize = 1;
pub const MULTI_WILDCARD_BYTES: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersion {
    pub major: ProtocolVersionSegment,
    pub minor: ProtocolVersionSegment,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientMessage {
    Get(Get),
    PGet(PGet),
    Set(Set),
    Subscribe(Subscribe),
    PSubscribe(PSubscribe),
    Export(Export),
    Import(Import),
    Unsubscribe(Unsubscribe),
}

impl ClientMessage {
    pub fn transaction_id(&self) -> TransactionId {
        match self {
            ClientMessage::Get(m) => m.transaction_id,
            ClientMessage::PGet(m) => m.transaction_id,
            ClientMessage::Set(m) => m.transaction_id,
            ClientMessage::Subscribe(m) => m.transaction_id,
            ClientMessage::PSubscribe(m) => m.transaction_id,
            ClientMessage::Export(m) => m.transaction_id,
            ClientMessage::Import(m) => m.transaction_id,
            ClientMessage::Unsubscribe(m) => m.transaction_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValuePair {
    pub key: Key,
    pub value: Value,
}

impl From<(String, String)> for KeyValuePair {
    fn from((key, value): (String, String)) -> Self {
        KeyValuePair { key, value }
    }
}

impl From<(&str, &str)> for KeyValuePair {
    fn from((key, value): (&str, &str)) -> Self {
        KeyValuePair {
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Get {
    pub transaction_id: TransactionId,
    pub key: Key,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PGet {
    pub transaction_id: TransactionId,
    pub request_pattern: RequestPattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Set {
    pub transaction_id: TransactionId,
    pub key: Key,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscribe {
    pub transaction_id: TransactionId,
    pub key: RequestPattern,
    pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PSubscribe {
    pub transaction_id: TransactionId,
    pub request_pattern: RequestPattern,
    pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PState {
    pub transaction_id: TransactionId,
    pub request_pattern: RequestPattern,
    pub key_value_pairs: KeyValuePairs,
}

impl fmt::Display for PState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kvps: Vec<String> = self
            .key_value_pairs
            .iter()
            .map(|&KeyValuePair { ref key, ref value }| format!("{key}={value}"))
            .collect();
        let joined = kvps.join("\n");
        write!(f, "{joined}")
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub transaction_id: TransactionId,
    pub key_value: KeyValuePair,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let KeyValuePair { key, value } = &self.key_value;
        write!(f, "{key}={value}")
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
    pub supported_protocol_versions: ProtocolVersions,
    pub separator: Separator,
    pub wildcard: Wildcard,
    pub multi_wildcard: MultiWildcard,
}

impl fmt::Display for Handshake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "handshake: separator: '{}', wildcard: '{}', multi-wildcard: '{}', supported protocol versions: {}",
            self.separator, self.wildcard, self.multi_wildcard, self.supported_protocol_versions.iter().map(|v| format!("{}.{}",v.major,v.minor)).collect::<Vec<String>>().join(", ")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Export {
    pub transaction_id: TransactionId,
    pub path: Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Import {
    pub transaction_id: TransactionId,
    pub path: Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unsubscribe {
    pub transaction_id: TransactionId,
}

pub fn encode_message(msg: &ClientMessage) -> EncodeResult<Vec<u8>> {
    match msg {
        ClientMessage::Get(msg) => encode_get_message(msg),
        ClientMessage::PGet(msg) => encode_pget_message(msg),
        ClientMessage::Set(msg) => encode_set_message(msg),
        ClientMessage::Subscribe(msg) => encode_subscribe_message(msg),
        ClientMessage::PSubscribe(msg) => encode_psubscribe_message(msg),
        ClientMessage::Export(msg) => encode_export_message(msg),
        ClientMessage::Import(msg) => encode_import_message(msg),
        ClientMessage::Unsubscribe(msg) => encode_unsubscribe_message(msg),
    }
}

pub fn encode_get_message(msg: &Get) -> EncodeResult<Vec<u8>> {
    let key_length = get_key_length(&msg.key)?;

    let mut buf = vec![GET];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(key_length.to_be_bytes());
    buf.extend(msg.key.as_bytes());

    Ok(buf)
}

pub fn encode_pget_message(msg: &PGet) -> EncodeResult<Vec<u8>> {
    let request_pattern_length = get_request_pattern_length(&msg.request_pattern)?;

    let mut buf = vec![PGET];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(request_pattern_length.to_be_bytes());
    buf.extend(msg.request_pattern.as_bytes());

    Ok(buf)
}

pub fn encode_set_message(msg: &Set) -> EncodeResult<Vec<u8>> {
    let key_length = get_key_length(&msg.key)?;
    let value_length = get_value_length(&msg.value)?;

    let mut buf = vec![SET];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(key_length.to_be_bytes());
    buf.extend(value_length.to_be_bytes());
    buf.extend(msg.key.as_bytes());
    buf.extend(msg.value.as_bytes());

    Ok(buf)
}

pub fn encode_subscribe_message(msg: &Subscribe) -> EncodeResult<Vec<u8>> {
    let key_length = get_key_length(&msg.key)?;

    let mut buf = vec![SUB];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(key_length.to_be_bytes());
    buf.extend(msg.key.as_bytes());
    buf.push(if msg.unique { 1 } else { 0 });

    Ok(buf)
}

pub fn encode_psubscribe_message(msg: &PSubscribe) -> EncodeResult<Vec<u8>> {
    let request_pattern_length = get_request_pattern_length(&msg.request_pattern)?;

    let mut buf = vec![PSUB];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(request_pattern_length.to_be_bytes());
    buf.extend(msg.request_pattern.as_bytes());
    buf.push(if msg.unique { 1 } else { 0 });

    Ok(buf)
}

pub fn encode_export_message(msg: &Export) -> EncodeResult<Vec<u8>> {
    let path_length = get_path_length(&msg.path)?;

    let mut buf = vec![EXP];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(path_length.to_be_bytes());
    buf.extend(msg.path.as_bytes());

    Ok(buf)
}

pub fn encode_import_message(msg: &Import) -> EncodeResult<Vec<u8>> {
    let path_length = get_path_length(&msg.path)?;

    let mut buf = vec![IMP];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(path_length.to_be_bytes());
    buf.extend(msg.path.as_bytes());

    Ok(buf)
}

pub fn encode_pstate_message(msg: &PState) -> EncodeResult<Vec<u8>> {
    let request_pattern_length = get_request_pattern_length(&msg.request_pattern)?;
    let num_key_val_pairs = get_num_key_val_pairs(&msg.key_value_pairs)?;

    let mut buf = vec![PSTA];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(request_pattern_length.to_be_bytes());
    buf.extend(num_key_val_pairs.to_be_bytes());

    for KeyValuePair { key, value } in &msg.key_value_pairs {
        let key_length = get_key_length(&key)?;
        let value_length = get_value_length(&value)?;
        buf.extend(key_length.to_be_bytes());
        buf.extend(value_length.to_be_bytes());
    }

    buf.extend(msg.request_pattern.as_bytes());

    for KeyValuePair { key, value } in &msg.key_value_pairs {
        buf.extend(key.as_bytes());
        buf.extend(value.as_bytes());
    }

    Ok(buf)
}

pub fn encode_ack_message(msg: &Ack) -> EncodeResult<Vec<u8>> {
    let mut buf = vec![ACK];

    buf.extend(msg.transaction_id.to_be_bytes());

    Ok(buf)
}

pub fn encode_state_message(msg: &State) -> EncodeResult<Vec<u8>> {
    let KeyValuePair { key, value } = &msg.key_value;
    let key_length = get_key_length(key)?;
    let value_length = get_value_length(value)?;

    let mut buf = vec![STA];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(key_length.to_be_bytes());
    buf.extend(value_length.to_be_bytes());

    buf.extend(key.as_bytes());
    buf.extend(value.as_bytes());

    Ok(buf)
}

pub fn encode_err_message(msg: &Err) -> EncodeResult<Vec<u8>> {
    let metadata_length = get_metadata_length(&msg.metadata)?;

    let mut buf = vec![ERR];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.push(msg.error_code);
    buf.extend(metadata_length.to_be_bytes());
    buf.extend(msg.metadata.as_bytes());

    Ok(buf)
}

pub fn encode_handshake_message(msg: &Handshake) -> EncodeResult<Vec<u8>> {
    let num_protocol_versions = get_num_protocol_versions(&msg.supported_protocol_versions)?;

    let mut buf = vec![HSHK];

    buf.extend(num_protocol_versions.to_be_bytes());

    for ProtocolVersion { major, minor } in &msg.supported_protocol_versions {
        buf.extend(major.to_be_bytes());
        buf.extend(minor.to_be_bytes());
    }

    buf.push(msg.separator as u8);
    buf.push(msg.wildcard as u8);
    buf.push(msg.multi_wildcard as u8);

    Ok(buf)
}

pub fn encode_unsubscribe_message(msg: &Unsubscribe) -> EncodeResult<Vec<u8>> {
    let mut buf = vec![USUB];

    buf.extend(msg.transaction_id.to_be_bytes());

    Ok(buf)
}

fn get_request_pattern_length(string: &str) -> EncodeResult<RequestPatternLength> {
    let length = string.len();
    if length > RequestPatternLength::MAX as usize {
        Err(EncodeError::RequestPatternTooLong(length))
    } else {
        Ok(length as RequestPatternLength)
    }
}

fn get_key_length(string: &str) -> EncodeResult<KeyLength> {
    let length = string.len();
    if length > KeyLength::MAX as usize {
        Err(EncodeError::KeyTooLong(length))
    } else {
        Ok(length as KeyLength)
    }
}

fn get_value_length(string: &str) -> EncodeResult<ValueLength> {
    let length = string.len();
    if length > ValueLength::MAX as usize {
        Err(EncodeError::ValueTooLong(length))
    } else {
        Ok(length as ValueLength)
    }
}

fn get_num_key_val_pairs(pairs: &KeyValuePairs) -> EncodeResult<NumKeyValuePairs> {
    let length = pairs.len();
    if length > NumKeyValuePairs::MAX as usize {
        Err(EncodeError::TooManyKeyValuePairs(length))
    } else {
        Ok(length as NumKeyValuePairs)
    }
}

fn get_num_protocol_versions(versions: &ProtocolVersions) -> EncodeResult<NumProtocolVersions> {
    let length = versions.len();
    if length > NumProtocolVersions::MAX as usize {
        Err(EncodeError::TooManyProtocolVersions(length))
    } else {
        Ok(length as NumProtocolVersions)
    }
}

fn get_metadata_length(string: &str) -> EncodeResult<MetaDataLength> {
    let length = string.len();
    if length > MetaDataLength::MAX as usize {
        Err(EncodeError::MetaDataTooLong(length))
    } else {
        Ok(length as MetaDataLength)
    }
}

fn get_path_length(string: &str) -> EncodeResult<PathLength> {
    let length = string.len();
    if length > PathLength::MAX as usize {
        Err(EncodeError::PathTooLong(length))
    } else {
        Ok(length as PathLength)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_message_is_encoded_correctly() {
        let msg = Get {
            transaction_id: 4,
            key: "trolo".to_owned(),
        };

        let data = vec![
            GET, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000100, 0b00000000, 0b00000101, b't', b'r', b'o', b'l', b'o',
        ];

        assert_eq!(data, encode_get_message(&msg).unwrap());
    }

    #[test]
    fn pget_message_is_encoded_correctly() {
        let msg = PGet {
            transaction_id: 4,
            request_pattern: "trolo".to_owned(),
        };

        let data = vec![
            PGET, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000100, 0b00000000, 0b00000101, b't', b'r', b'o', b'l', b'o',
        ];

        assert_eq!(data, encode_pget_message(&msg).unwrap());
    }

    #[test]
    fn set_message_is_encoded_correctly() {
        let msg = Set {
            transaction_id: 0,
            key: "yo/mama".to_owned(),
            value: "fat".to_owned(),
        };

        let data = vec![
            SET, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000000, 0b00000111, 0b00000000, 0b00000000, 0b00000000,
            0b00000011, b'y', b'o', b'/', b'm', b'a', b'm', b'a', b'f', b'a', b't',
        ];

        assert_eq!(data, encode_set_message(&msg).unwrap());
    }

    #[test]
    fn subscribe_message_is_encoded_correctly() {
        let msg = Subscribe {
            transaction_id: 5536684732567,
            key: "let/me/?/you/its/features".to_owned(),
            unique: true,
        };

        let data = vec![
            SUB, 0b00000000, 0b00000000, 0b00000101, 0b00001001, 0b00011100, 0b00100000,
            0b01110000, 0b10010111, 0b00000000, 0b00011001, b'l', b'e', b't', b'/', b'm', b'e',
            b'/', b'?', b'/', b'y', b'o', b'u', b'/', b'i', b't', b's', b'/', b'f', b'e', b'a',
            b't', b'u', b'r', b'e', b's', 0b00000001,
        ];

        assert_eq!(data, encode_subscribe_message(&msg).unwrap());
    }

    #[test]
    fn psubscribe_message_is_encoded_correctly() {
        let msg = PSubscribe {
            transaction_id: 5536684732567,
            request_pattern: "let/me/?/you/its/features".to_owned(),
            unique: false,
        };

        let data = vec![
            PSUB, 0b00000000, 0b00000000, 0b00000101, 0b00001001, 0b00011100, 0b00100000,
            0b01110000, 0b10010111, 0b00000000, 0b00011001, b'l', b'e', b't', b'/', b'm', b'e',
            b'/', b'?', b'/', b'y', b'o', b'u', b'/', b'i', b't', b's', b'/', b'f', b'e', b'a',
            b't', b'u', b'r', b'e', b's', 0b00000000,
        ];

        assert_eq!(data, encode_psubscribe_message(&msg).unwrap());
    }

    #[test]
    fn handshake_message_is_encoded_correctly() {
        let msg = Handshake {
            supported_protocol_versions: vec![
                ProtocolVersion { major: 1, minor: 0 },
                ProtocolVersion { major: 1, minor: 1 },
                ProtocolVersion { major: 1, minor: 2 },
            ],
            separator: '/',
            wildcard: '?',
            multi_wildcard: '#',
        };

        let data = vec![
            HSHK, 0b00000011, 0b00000000, 0b00000001, 0b00000000, 0b00000000, 0b00000000,
            0b00000001, 0b00000000, 0b00000001, 0b00000000, 0b00000001, 0b00000000, 0b00000010,
            b'/', b'?', b'#',
        ];

        assert_eq!(data, encode_handshake_message(&msg).unwrap());
    }

    #[test]
    fn pstate_message_is_encoded_correctly() {
        let msg = PState {
            transaction_id: u64::MAX,
            request_pattern: "who/let/the/?/#".to_owned(),
            key_value_pairs: vec![
                (
                    "who/let/the/chicken/cross/the/road".to_owned(),
                    "yeah, that was me, I guess".to_owned(),
                )
                    .into(),
                (
                    "who/let/the/dogs/out".to_owned(),
                    "Who? Who? Who? Who? Who?".to_owned(),
                )
                    .into(),
            ],
        };

        let data = vec![
            PSTA, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b11111111,
            0b11111111, 0b11111111, 0b00000000, 0b00001111, 0b00000000, 0b00000000, 0b00000000,
            0b00000010, 0b00000000, 0b00100010, 0b00000000, 0b00000000, 0b00000000, 0b00011010,
            0b00000000, 0b00010100, 0b00000000, 0b00000000, 0b00000000, 0b00011000, b'w', b'h',
            b'o', b'/', b'l', b'e', b't', b'/', b't', b'h', b'e', b'/', b'?', b'/', b'#', b'w',
            b'h', b'o', b'/', b'l', b'e', b't', b'/', b't', b'h', b'e', b'/', b'c', b'h', b'i',
            b'c', b'k', b'e', b'n', b'/', b'c', b'r', b'o', b's', b's', b'/', b't', b'h', b'e',
            b'/', b'r', b'o', b'a', b'd', b'y', b'e', b'a', b'h', b',', b' ', b't', b'h', b'a',
            b't', b' ', b'w', b'a', b's', b' ', b'm', b'e', b',', b' ', b'I', b' ', b'g', b'u',
            b'e', b's', b's', b'w', b'h', b'o', b'/', b'l', b'e', b't', b'/', b't', b'h', b'e',
            b'/', b'd', b'o', b'g', b's', b'/', b'o', b'u', b't', b'W', b'h', b'o', b'?', b' ',
            b'W', b'h', b'o', b'?', b' ', b'W', b'h', b'o', b'?', b' ', b'W', b'h', b'o', b'?',
            b' ', b'W', b'h', b'o', b'?',
        ];

        assert_eq!(data, encode_pstate_message(&msg).unwrap());
    }

    #[test]
    fn ack_message_is_encoded_correctly() {
        let msg = Ack { transaction_id: 42 };

        let data = vec![
            ACK, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00101010,
        ];

        assert_eq!(data, encode_ack_message(&msg).unwrap());
    }

    #[test]
    fn state_message_is_encoded_correctly() {
        let msg = State {
            transaction_id: 42,
            key_value: ("1/2/3", "4").into(),
        };

        let data = vec![
            STA, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00101010, 0b00000000, 0b00000101, 0b00000000, 0b00000000, 0b00000000,
            0b00000001, b'1', b'/', b'2', b'/', b'3', b'4',
        ];

        assert_eq!(data, encode_state_message(&msg).unwrap());
    }

    #[test]
    fn empty_state_message_is_encoded_correctly() {
        let msg = State {
            transaction_id: 42,
            key_value: ("1/2/3", "").into(),
        };

        let data = vec![
            STA, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00101010, 0b00000000, 0b00000101, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, b'1', b'/', b'2', b'/', b'3',
        ];

        assert_eq!(data, encode_state_message(&msg).unwrap());
    }

    #[test]
    fn err_message_is_encoded_correctly() {
        let msg = Err {
            transaction_id: 42,
            error_code: 5,
            metadata: "THIS IS METAAA!!!".to_owned(),
        };

        let data = vec![
            ERR, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00101010, 0b00000101, 0b00000000, 0b00000000, 0b00000000, 0b00010001,
            b'T', b'H', b'I', b'S', b' ', b'I', b'S', b' ', b'M', b'E', b'T', b'A', b'A', b'A',
            b'!', b'!', b'!',
        ];

        assert_eq!(data, encode_err_message(&msg).unwrap());
    }

    #[test]
    fn export_message_is_encoded_correctly() {
        let msg = Export {
            transaction_id: 42,
            path: "/path/to/file".to_owned(),
        };

        let data = vec![
            EXP, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00101010, 0b00000000, 0b00001101, b'/', b'p', b'a', b't', b'h', b'/',
            b't', b'o', b'/', b'f', b'i', b'l', b'e',
        ];

        assert_eq!(data, encode_export_message(&msg).unwrap());
    }

    #[test]
    fn import_message_is_encoded_correctly() {
        let msg = Import {
            transaction_id: 42,
            path: "/path/to/file".to_owned(),
        };

        let data = vec![
            IMP, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00101010, 0b00000000, 0b00001101, b'/', b'p', b'a', b't', b'h', b'/',
            b't', b'o', b'/', b'f', b'i', b'l', b'e',
        ];

        assert_eq!(data, encode_import_message(&msg).unwrap());
    }

    #[test]
    fn unsubscribe_message_is_encoded_correctly() {
        let msg = Unsubscribe { transaction_id: 42 };

        let data = vec![
            USUB, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00101010,
        ];

        assert_eq!(data, encode_unsubscribe_message(&msg).unwrap());
    }
}