import React, { Component } from "react";
import { connect } from "react-redux";
import { hot } from "react-hot-loader/root";

import Sidebar from "./components/Sidebar";
import ChatList from "./components/ChatList";
import Chat from "./components/Chat";
import { maybeNetwork } from "./redux";

class App extends Component {
  componentDidMount() {
    window.addEventListener("offline", this.offlineEvent);
    window.addEventListener("online", this.onlineEvent);
  }

  componentWillUnmount() {
    window.removeEventListener("offline", this.offlineEvent);
    window.addEventListener("online", this.onlineEvent);
  }

  offlineEvent = event => {
    console.log("OFFLINE");
  };

  onlineEvent = event => {
    console.log("ONLINE");
    this.props.maybeNetwork();
  };

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

const mapDispatchToProps = {
  maybeNetwork
};

export default hot(connect(mapStateToProps, mapDispatchToProps)(App));
