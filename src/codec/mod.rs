#[cfg(feature = "async")]
mod nonblocking;
#[cfg(feature = "async")]
pub use nonblocking::*;

#[cfg(not(feature = "async"))]
mod blocking;
#[cfg(not(feature = "async"))]
pub use blocking::*;

use crate::error::{EncodeError, EncodeResult};
use serde::{Deserialize, Serialize};

pub type MessageType = u8;
pub type TransactionId = u64;
pub type RequestPattern = String;
pub type Key = String;
pub type Value = String;
pub type KeyValuePairs = Vec<(String, String)>;
pub type ErrorCode = u8;
pub type MetaData = String;

pub type RequestPatternLength = u16;
pub type KeyLength = u16;
pub type ValueLength = u32;
pub type MetaDataLength = u32;
pub type NumKeyValuePairs = u32;

pub const GET: MessageType = 0b00000000;
pub const SET: MessageType = 0b00000001;
pub const SUB: MessageType = 0b00000010;

pub const STA: MessageType = 0b10000000;
pub const ACK: MessageType = 0b10000001;
pub const EVE: MessageType = 0b10000010;
pub const ERR: MessageType = 0b10000011;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Message {
    // client messages
    Get(Get),
    Set(Set),
    Subscribe(Subscribe),
    // server messages
    State(State),
    Ack(Ack),
    Event(Event),
    Err(Err),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Get {
    pub transaction_id: TransactionId,
    pub request_pattern: RequestPattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Set {
    pub transaction_id: TransactionId,
    pub key: Key,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Subscribe {
    pub transaction_id: TransactionId,
    pub request_pattern: RequestPattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
    pub transaction_id: TransactionId,
    pub request_pattern: RequestPattern,
    pub key_value_pairs: KeyValuePairs,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ack {
    pub transaction_id: TransactionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Event {
    pub transaction_id: TransactionId,
    pub request_pattern: RequestPattern,
    pub key: Key,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Err {
    pub transaction_id: TransactionId,
    pub error_code: ErrorCode,
    pub metadata: MetaData,
}

pub fn encode_get_message(msg: &Get) -> EncodeResult<Vec<u8>> {
    let request_pattern_length = get_request_pattern_length(&msg.request_pattern)?;

    let mut buf = vec![GET];

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
    let request_pattern_length = get_request_pattern_length(&msg.request_pattern)?;

    let mut buf = vec![SUB];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(request_pattern_length.to_be_bytes());
    buf.extend(msg.request_pattern.as_bytes());

    Ok(buf)
}

pub fn encode_state_message(msg: &State) -> EncodeResult<Vec<u8>> {
    let request_pattern_length = get_request_pattern_length(&msg.request_pattern)?;
    let num_key_val_pairs = get_num_key_val_pairs(&msg.key_value_pairs)?;

    let mut buf = vec![STA];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(request_pattern_length.to_be_bytes());
    buf.extend(num_key_val_pairs.to_be_bytes());

    for (key, value) in &msg.key_value_pairs {
        let key_length = get_key_length(&key)?;
        let value_length = get_value_length(&value)?;
        buf.extend(key_length.to_be_bytes());
        buf.extend(value_length.to_be_bytes());
    }

    buf.extend(msg.request_pattern.as_bytes());

    for (key, value) in &msg.key_value_pairs {
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

pub fn encode_event_message(msg: &Event) -> EncodeResult<Vec<u8>> {
    let request_pattern_length = get_request_pattern_length(&msg.request_pattern)?;
    let key_length = get_key_length(&msg.key)?;
    let value_length = get_value_length(&msg.value)?;

    let mut buf = vec![EVE];

    buf.extend(msg.transaction_id.to_be_bytes());
    buf.extend(request_pattern_length.to_be_bytes());
    buf.extend(key_length.to_be_bytes());
    buf.extend(value_length.to_be_bytes());
    buf.extend(msg.request_pattern.as_bytes());
    buf.extend(msg.key.as_bytes());
    buf.extend(msg.value.as_bytes());

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

fn get_num_key_val_pairs(pairs: &[(String, String)]) -> EncodeResult<NumKeyValuePairs> {
    let length = pairs.len();
    if length > NumKeyValuePairs::MAX as usize {
        Err(EncodeError::TooManyKeyValuePairs(length))
    } else {
        Ok(length as NumKeyValuePairs)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_message_is_encoded_correctly() {
        let msg = Get {
            transaction_id: 4,
            request_pattern: "trolo".to_owned(),
        };

        let data = vec![
            GET, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000100, 0b00000000, 0b00000101, b't', b'r', b'o', b'l', b'o',
        ];

        assert_eq!(data, encode_get_message(&msg).unwrap());
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
            request_pattern: "let/me/?/you/its/features".to_owned(),
        };

        let data = vec![
            SUB, 0b00000000, 0b00000000, 0b00000101, 0b00001001, 0b00011100, 0b00100000,
            0b01110000, 0b10010111, 0b00000000, 0b00011001, b'l', b'e', b't', b'/', b'm', b'e',
            b'/', b'?', b'/', b'y', b'o', b'u', b'/', b'i', b't', b's', b'/', b'f', b'e', b'a',
            b't', b'u', b'r', b'e', b's',
        ];

        assert_eq!(data, encode_subscribe_message(&msg).unwrap());
    }

    #[test]
    fn state_message_is_encoded_correctly() {
        let msg = State {
            transaction_id: u64::MAX,
            request_pattern: "who/let/the/?/#".to_owned(),
            key_value_pairs: vec![
                (
                    "who/let/the/chicken/cross/the/road".to_owned(),
                    "yeah, that was me, I guess".to_owned(),
                ),
                (
                    "who/let/the/dogs/out".to_owned(),
                    "Who? Who? Who? Who? Who?".to_owned(),
                ),
            ],
        };

        let data = vec![
            STA, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b11111111,
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

        assert_eq!(data, encode_state_message(&msg).unwrap());
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
    fn event_message_is_encoded_correctly() {
        let msg = Event {
            transaction_id: 42,
            request_pattern: "1/2/3".to_owned(),
            key: "1/2/3".to_owned(),
            value: "4".to_owned(),
        };

        let data = vec![
            EVE, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00101010, 0b00000000, 0b00000101, 0b00000000, 0b00000101, 0b00000000,
            0b00000000, 0b00000000, 0b00000001, b'1', b'/', b'2', b'/', b'3', b'1', b'/', b'2',
            b'/', b'3', b'4',
        ];

        assert_eq!(data, encode_event_message(&msg).unwrap());
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
}