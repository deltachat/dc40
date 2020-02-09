import React from "react";
import ReactDOM from "react-dom";
import { createStore, compose, applyMiddleware } from "redux";
import { Provider } from "react-redux";
import ReactModal from "react-modal";

import "modern-css-reset";
import "react-virtualized/styles.css";

import "./style.less";

import App from "./App";
import { reducer } from "./redux/index";
import { remoteMiddleware } from "./redux/remote";

ReactModal.setAppElement("#root");

const composer = window.__REDUX_DEVTOOLS_EXTENSION_COMPOSE__ || compose;

const store = createStore(
  reducer,
  composer(applyMiddleware(remoteMiddleware("ws://localhost:8080")))
);

ReactDOM.render(
  <Provider store={store}>
    <App />
  </Provider>,
  document.getElementById("root")
);
