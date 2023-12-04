use std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
pub enum WSMSGMessageTypeC2S {
    Message,
    Ping,
    ListSubscribed,
    Subscribe,
    Unsubscribe,
}

#[allow(dead_code)]
impl WSMSGMessageTypeC2S {
    pub fn from_message_type(value: u8) -> Option<Self> {
        return match value {
            255u8 => Some(WSMSGMessageTypeC2S::Message),
            0u8 => Some(WSMSGMessageTypeC2S::Ping),
            1u8 => Some(WSMSGMessageTypeC2S::ListSubscribed),
            2u8 => Some(WSMSGMessageTypeC2S::Subscribe),
            3u8 => Some(WSMSGMessageTypeC2S::Unsubscribe),
            _ => None,
        };
    }

    pub fn to_message_type(value: Self) -> u8 {
        return match value {
            WSMSGMessageTypeC2S::Message => 255u8,
            WSMSGMessageTypeC2S::Ping => 0u8,
            WSMSGMessageTypeC2S::ListSubscribed => 1u8,
            WSMSGMessageTypeC2S::Subscribe => 2u8,
            WSMSGMessageTypeC2S::Unsubscribe => 3u8,
        };
    }
}

impl std::fmt::Display for WSMSGMessageTypeC2S {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        return match self {
            WSMSGMessageTypeC2S::Message => fmt.write_str("C2S_Message"),
            WSMSGMessageTypeC2S::Ping => fmt.write_str("C2S_Ping"),
            WSMSGMessageTypeC2S::ListSubscribed => fmt.write_str("C2S_ListSubscribed"),
            WSMSGMessageTypeC2S::Subscribe => fmt.write_str("C2S_Subscribe"),
            WSMSGMessageTypeC2S::Unsubscribe => fmt.write_str("C2S_Unsubscribe"),
        };
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WSMSGMessageTypeS2C {
    Message,
    Reply,
}

#[allow(dead_code)]
impl WSMSGMessageTypeS2C {
    pub fn from_message_type(value: u8) -> Option<Self> {
        return match value {
            255u8 => Some(WSMSGMessageTypeS2C::Message),
            0u8 => Some(WSMSGMessageTypeS2C::Reply),
            _ => None,
        };
    }

    pub fn to_message_type(value: Self) -> u8 {
        return match value {
            WSMSGMessageTypeS2C::Message => 255u8,
            WSMSGMessageTypeS2C::Reply => 0u8,
        };
    }
}

impl std::fmt::Display for WSMSGMessageTypeS2C {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        return match self {
            WSMSGMessageTypeS2C::Message => fmt.write_str("S2C_Message"),
            WSMSGMessageTypeS2C::Reply => fmt.write_str("S2C_Reply"),
        };
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WSMSGMessageError {
    MessageMalformed,
    InvalidMessageType,
}

impl std::fmt::Display for WSMSGMessageError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        return write!(fmt, "{:?}", self);
    }
}

#[derive(Debug, Clone)]
pub struct WSMSGMessage {
    pub message_type: WSMSGMessageTypeC2S,
    pub message_id: u64,
    pub message_data: Vec<u8>,
}

impl WSMSGMessage {
    pub fn from_message(bytes: Vec<u8>) -> Result<WSMSGMessage, WSMSGMessageError> {
        if bytes.len() < 9 {
            return Err(WSMSGMessageError::MessageMalformed);
        }

        let message_type = WSMSGMessageTypeC2S::from_message_type(bytes[0]);
        if message_type.is_none() {
            return Err(WSMSGMessageError::InvalidMessageType);
        }

        let message_type = message_type.unwrap();

        let message_id = u64::from_be_bytes(bytes[1..9].try_into().unwrap());

        let message_data = bytes[9..].to_vec();
        Ok(WSMSGMessage {
            message_type,
            message_id,
            message_data,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WSMSGResponse {
    pub message_type: WSMSGMessageTypeS2C,
    pub message_id: Option<u64>,
    pub message_code: Option<u8>,
    pub message_data: Vec<u8>,
}

impl WSMSGResponse {
    pub fn to_message(&self) -> Vec<u8> {
        let size: usize =
            1 + self.message_data.len() + if self.message_id.is_some() { 8 } else { 0 };

        let mut message: Vec<u8> = vec![];
        message.reserve_exact(size);

        message.push(WSMSGMessageTypeS2C::to_message_type(self.message_type));

        if let Some(id) = self.message_id {
            message.extend_from_slice(&id.to_be_bytes());
        }

        if let Some(code) = self.message_code {
            message.push(code);
        }

        message.extend(self.message_data.iter());

        return message;
    }
}
