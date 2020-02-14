import React, { Component } from "react";
import { connect } from "react-redux";
import { hot } from "react-hot-loader/root";
import { Titlebar } from "react-titlebar-osx";

const { api } = window;

import AccountList from "./components/AccountList";
import ChatList from "./components/ChatList";
import Chat from "./components/Chat";

class App extends Component {
  constructor(props) {
    super(props);

    this.state = {
      isFullscreen: false
    };
  }

  onClose = () => {
    api.send("toMain", "close");
  };

  onMinimize = () => {
    api.send("toMain", "minimize");
  };

  onMaximize = () => {
    api.send("toMain", "maximize");
  };

  onFullscreen = () => {
    if (this.state.isFullscreen) {
      api.send("toMain", "exit-full-screen");
    } else {
      api.send("toMain", "enter-full-screen");
    }

    this.setState({ isFullscreen: !this.state.isFullscreen });
  };

  render() {
    return (
      <div className="app">
        <div className="account-header">
          <Titlebar
            draggable={true}
            transparent={true}
            padding={12}
            onFullscreen={this.onFullscreen}
            onMaximize={this.onMaximize}
            onMinimize={this.onMinimize}
            onClose={this.onClose}
          />
          <div className="account-info">{this.props.selectedAccount}</div>
        </div>
        <div className="app-container">
          <AccountList />
          <ChatList />
          <Chat />
        </div>
      </div>
    );
  }
}

const mapStateToProps = state => ({
  selectedAccount: state.shared.selected_account
});

const mapDispatchToProps = {};

export default hot(connect(mapStateToProps, mapDispatchToProps)(App));
