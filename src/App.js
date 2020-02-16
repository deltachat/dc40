import React, { Component } from "react";
import { connect } from "react-redux";
import { hot } from "react-hot-loader/root";

import Sidebar from "./components/Sidebar";
import ChatList from "./components/ChatList";
import Chat from "./components/Chat";

class App extends Component {
  render() {
    return (
      <div className="app">
        <Sidebar />
        <ChatList />
        <Chat />
      </div>
    );
  }
}

const mapStateToProps = state => ({
  selectedAccount: state.shared.selected_account
});

const mapDispatchToProps = {};

export default hot(connect(mapStateToProps, mapDispatchToProps)(App));
