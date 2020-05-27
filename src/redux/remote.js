import { combineReducers } from "redux";
import ReconnectingWebSocket from "reconnecting-websocket";

export const remoteAction = action => {
  action.remote = true;
  return action;
};

const remoteReducer = (reducer, name) => {
  return (state, action) => {
    if (action.type === "REMOTE_UPDATE") {
      return action.state[name];
    } else {
      return reducer(state, action);
    }
  };
};

export const remoteCombineReducers = (localReducers, remoteReducers) => {
  Object.keys(remoteReducers).forEach(key => {
    remoteReducers[key] = remoteReducer(remoteReducers[key], key);
  });

  return combineReducers({ ...localReducers, ...remoteReducers });
};

export const remoteMiddleware = url => store => {
  const dispatch = remoteDispatch(url, store.dispatch);
  return next => action => {
    if (action && action.remote) {
      dispatch(action);
    } else {
      next(action);
    }
  };
};

const remoteDispatch = (url, localDispatch) => {
  const ws = new ReconnectingWebSocket(url);

  ws.addEventListener("message", message => {
    localDispatch(JSON.parse(message.data));
  });

  return action => ws.send(JSON.stringify(action));
};
