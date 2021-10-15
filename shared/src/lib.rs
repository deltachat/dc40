use std::collections::{HashMap, HashSet};
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
    Account {
        account: u32,
        chat: Option<ChatState>,
        chat_id: Option<u32>,
    },
    Event {
        account: u32,
        event: Event,
    },
    Contacts(Vec<ContactInfo>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ContactInfo {
    pub id: u32,
    pub display_name: String,
    pub mail: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Event {
    Configure(Progress),
    Imex(Progress),
    Connected,
    MessagesChanged {
        chat_id: u32,
    },
    MessageIncoming {
        chat_id: u32,
        title: String,
        body: String,
    },
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
    pub accounts: HashMap<u32, SharedAccountState>,
    pub errors: Vec<String>,
    pub selected_account: Option<u32>,
    pub selected_chat_id: Option<u32>,
    pub selected_chat: Option<ChatState>,
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub enum ChatItem {
    Message(u32),
    DayMarker(DateTime<Utc>),
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub enum ChatMessage {
    Message(InnerChatMessage),
    DayMarker(DateTime<Utc>),
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub struct InnerChatMessage {
    pub id: u32,
    pub from_id: u32,
    pub from_first_name: String,
    pub from_profile_image: Option<PathBuf>,
    pub from_color: u32,
    pub viewtype: Viewtype,
    pub state: String,
    pub text: Option<String>,
    pub quote: Option<Box<InnerChatMessage>>,
    pub timestamp: DateTime<Utc>,
    pub is_info: bool,
    pub file: Option<PathBuf>,
    pub file_height: i32,
    pub file_width: i32,
    pub is_first: bool,
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
    pub is_contact_request: bool,
    pub is_self_talk: bool,
    pub is_device_talk: bool,
    pub chat_type: String,
    pub color: u32,
    pub member_count: usize,
    pub is_pinned: bool,
    pub is_archived: bool,
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
    pub profile_image: Option<PathBuf>,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Login {
        email: String,
        password: String,
    },
    SelectChat {
        account: u32,
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
        account: u32,
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
    MaybeNetwork,
    AcceptContactRequest {
        account: u32,
        chat_id: u32,
    },
    BlockContact {
        account: u32,
        chat_id: u32,
    },
    GetAccountDetail {
        id: u32,
    },
    PinChat {
        account: u32,
        chat_id: u32,
    },
    UnpinChat {
        account: u32,
        chat_id: u32,
    },
    ArchiveChat {
        account: u32,
        chat_id: u32,
    },
    UnarchiveChat {
        account: u32,
        chat_id: u32,
    },
    GetContacts,
    CreateChat(HashSet<u32>),
    CreateGroupChat(HashSet<u32>, String),
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
