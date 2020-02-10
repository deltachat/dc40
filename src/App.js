import React, { Component } from "react";
import { connect } from "react-redux";
import { hot } from "react-hot-loader/root";

import Message from "./components/Message";
import AccountList from "./components/AccountList";
import ChatList from "./components/ChatList";
import Chat from "./components/Chat";

class App extends Component {
  render() {
    return (
      <div className="app">
        <AccountList />
        <ChatList />
        <Chat />
      </div>
    );
  }
}

const mapStateToProps = state => ({
  accounts: state.shared.accounts,
  errors: state.shared.errors
});

const mapDispatchToProps = {};

export default hot(connect(mapStateToProps, mapDispatchToProps)(App));
