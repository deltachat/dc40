import { remoteAction, remoteCombineReducers } from './remote';

const foo = (state = { value: 0, }, action) => {
    switch (action.type) {
        case 'ADD':
            return { value: state.value + action.value };
        default:
            return state;
    }
};

const DEFAULT_SHARED_STATE = {
  messages: [],
  details: {},
  accounts: [],
  errors: [],
}

const shared = (state = DEFAULT_SHARED_STATE, action) => {
  switch (action.type) {
  case 'SAY':
    return { messages: [...state.messages, action.message] }
  case 'LOG':
    console.log(action.event)
    return state
  default:
    return state
  }
}

export const reducer = remoteCombineReducers({ foo }, { shared });

export const say = (message) => remoteAction({
  type: 'INFO',
  message,
})

export const login = (email, password) => remoteAction({
  type: 'LOGIN',
  email,
  password
})

export const selectChat = (account, chat_id) => remoteAction({
  type: 'SELECT_CHAT',
  account,
  chat_id: parseInt(chat_id, 10),
})

export const imex = (email, path) => remoteAction({
  type: 'IMPORT',
  email,
  path
})

export const add = (value) => ({
  type: 'ADD',
  value,
})


