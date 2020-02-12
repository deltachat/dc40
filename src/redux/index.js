import { remoteAction, remoteCombineReducers } from "./remote";

const foo = (state = { value: 0 }, action) => {
  switch (action.type) {
    case "ADD":
      return { value: state.value + action.value };
    default:
      return state;
  }
};

const DEFAULT_SHARED_STATE = {
  messages: [],
  details: {},
  accounts: [],
  errors: []
};

const shared = (state = DEFAULT_SHARED_STATE, action) => {
  switch (action.type) {
    case "LOG":
      console.log(action.event);
      return state;
    default:
      return state;
  }
};

export const reducer = remoteCombineReducers({ foo }, { shared });

export const login = (email, password) =>
  remoteAction({
    type: "LOGIN",
    email,
    password
  });

export const selectChat = (account, chat_id) =>
  remoteAction({
    type: "SELECT_CHAT",
    account,
    chat_id: parseInt(chat_id, 10)
  });

export const selectAccount = account =>
  remoteAction({
    type: "SELECT_ACCOUNT",
    account
  });

export const loadChatList = (start_index, stop_index) =>
  remoteAction({
    type: "LOAD_CHAT_LIST",
    start_index,
    stop_index
  });

export const loadMessageList = (start_index, stop_index) =>
  remoteAction({
    type: "LOAD_MESSAGE_LIST",
    start_index,
    stop_index
  });

export const imex = (email, path) =>
  remoteAction({
    type: "IMPORT",
    email,
    path
  });

export const sendTextMessage = text =>
  remoteAction({
    type: "SEND_TEXT_MESSAGE",
    text
  });

export const sendFileMessage = (typ, path, text, mime) =>
  remoteAction({
    type: "SEND_FILE_MESSAGE",
    typ,
    path,
    text,
    mime
  });

export const add = value => ({
  type: "ADD",
  value
});
