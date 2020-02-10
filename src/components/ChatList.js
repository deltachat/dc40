import React from "react";
import { connect } from "react-redux";
import { List, AutoSizer } from "react-virtualized";

import { selectChat } from "../redux/index";

class ChatList extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      email: "",
      password: ""
    };
  }

  onChatClick(id, event) {
    event.preventDefault();
    this.props.selectChat(this.props.selectedAccount, id);
  }

  rowRenderer = ({ index, isScrolling, isVisible, key, parent, style }) => {
    const { account, selectedChat } = this.props;
    if (account == null) {
      return <div key={key} style={style}></div>;
    }

    const id = Object.keys(account.chat_states)[index];
    const chat_state = account.chat_states[id];
    const chat = account.chats[id];

    let image = <div className="letter-icon">{chat.name[0]}</div>;
    if (chat_state.profile_image != null) {
      image = (
        <img
          className="image-icon"
          src={"dc://" + chat_state.profile_image}
          alt="chat avatar"
        />
      );
    }

    let className = "chat-list-item";

    if (selectedChat === parseInt(id, 10)) {
      className += " active";
    }

    return (
      <div
        className={className}
        key={key}
        style={style}
        onClick={this.onChatClick.bind(this, id)}
      >
        <div className="chat-icon">{image}</div>
        <div className="chat-content">
          <div className="chat-header">{chat.name}</div>
          <div className="chat-preview">{chat_state.preview}</div>
        </div>
      </div>
    );
  };

  render() {
    let { account } = this.props;

    if (account == null) {
      return <div>Please login</div>;
    }

    let rowCount = account.chat_states
      ? Object.keys(account.chat_states).length
      : 0;

    return (
      <AutoSizer disableWidth>
        {({ height }) => (
          <List
            className="chat-list"
            height={height}
            rowCount={rowCount}
            rowHeight={80}
            rowRenderer={this.rowRenderer}
            width={350}
            {
              ...this.props /* Force rerender when props change*/
            }
          />
        )}
      </AutoSizer>
    );
  }
}

const mapStateToProps = state => {
  let selected = state.shared.selected_account;
  let accounts = state.shared.accounts;
  let account = selected != null && accounts != null && accounts[selected];

  return {
    account,
    selectedAccount: selected,
    selectedChat: account && account.selected_chat
  };
};

const mapDispatchToProps = {
  selectChat
};

export default connect(mapStateToProps, mapDispatchToProps)(ChatList);
