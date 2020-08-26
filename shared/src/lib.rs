use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use num_derive::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    RemoteUpdate {
        state: State,
    },
    MessageList {
        chat_id: u32,
        range: (usize, usize),
        items: Vec<ChatItem>,
        messages: Vec<ChatMessage>,
    },
    ChatList {
        range: (usize, usize),
        len: usize,
        chats: Vec<ChatState>,
    },
    Event {
        account: u32,
        event: Event,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Event {
    Configure(Progress),
    Imex(Progress),
    Connected,
    MessagesChanged { chat_id: u32 },
    MessageIncoming { chat_id: u32 },
    Log(Log),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Progress {
    Success,
    Error,
    Step(usize),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Log {
    Info(String),
    Warning(String),
    Error(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub shared: SharedState,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SharedState {
    pub accounts: HashMap<String, SharedAccountState>,
    pub errors: Vec<String>,
    pub selected_account: Option<String>,
    pub selected_chat_id: Option<u32>,
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub enum ChatItem {
    Message(u32),
    DayMarker(DateTime<Utc>),
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub enum ChatMessage {
    Message {
        id: u32,
        from_id: u32,
        from_first_name: String,
        from_profile_image: Option<PathBuf>,
        from_color: u32,
        viewtype: Viewtype,
        state: String,
        text: Option<String>,
        starred: bool,
        timestamp: DateTime<Utc>,
        is_info: bool,
        file: Option<PathBuf>,
        file_height: i32,
        file_width: i32,
        is_first: bool,
    },
    DayMarker(DateTime<Utc>),
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq, Eq)]
pub struct ChatState {
    pub index: Option<usize>,
    pub id: u32,
    pub name: String,
    pub header: String,
    pub preview: String,
    pub timestamp: DateTime<Utc>,
    pub state: String,
    pub profile_image: Option<PathBuf>,
    pub fresh_msg_cnt: usize,
    pub can_send: bool,
    pub is_self_talk: bool,
    pub is_device_talk: bool,
    pub chat_type: String,
    pub color: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Login {
    Success,
    Error(String),
    Progress(usize),
    Not,
}

impl Default for Login {
    fn default() -> Self {
        Login::Not
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, Eq)]
pub struct SharedAccountState {
    pub logged_in: Login,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Login {
        email: String,
        password: String,
        remote: bool,
    },
    Import {
        path: String,
        email: String,
    },
    SelectChat {
        account: String,
        chat_id: u32,
    },
    LoadChatList {
        start_index: usize,
        stop_index: usize,
    },
    LoadMessageList {
        start_index: usize,
        stop_index: usize,
    },
    SelectAccount {
        account: String,
    },
    SendTextMessage {
        text: String,
    },
    SendFileMessage {
        typ: Viewtype,
        path: String,
        text: Option<String>,
        mime: Option<String>,
    },
    CreateChatById {
        id: u32,
    },
    MaybeNetwork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum Viewtype {
    Unknown = 0,
    Text = 10,
    Image = 20,
    Gif = 21,
    Sticker = 23,
    Audio = 40,
    Voice = 41,
    Video = 50,
    File = 60,
    VideochatInvitation = 70,
}
