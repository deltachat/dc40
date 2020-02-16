import React from "react";
import { Titlebar } from "react-titlebar-osx";

const { api } = window;

import AccountList from "./AccountList";

class Sidebar extends React.Component {
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
      <div className="sidebar">
        <Titlebar
          draggable={true}
          transparent={true}
          padding={12}
          onFullscreen={this.onFullscreen}
          onMaximize={this.onMaximize}
          onMinimize={this.onMinimize}
          onClose={this.onClose}
        />

        <AccountList />
      </div>
    );
  }
}

export default Sidebar;
