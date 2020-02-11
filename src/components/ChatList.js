import React from "react";
import { connect } from "react-redux";
import { InfiniteLoader, List, AutoSizer } from "react-virtualized";

import { selectChat, loadChatList } from "../redux/index";

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
    if (this.props.selectedChatId != id) {
      this.props.selectChat(this.props.selectedAccount, id);
    }
  }

  rowRenderer = ({ index, isScrolling, isVisible, key, parent, style }) => {
    const { selectedAccount, selectedChatId, chats } = this.props;
    const chat = chats[index];

    if (selectedAccount == null || chat == null) {
      return <div key={key} style={style}></div>;
    }

    const id = chat.id;

    let image = <div className="letter-icon">{chat.name[0]}</div>;
    if (chat.profile_image != null) {
      image = (
        <img
          className="image-icon"
          src={"dc://" + chat.profile_image}
          alt="chat avatar"
        />
      );
    }

    let className = "chat-list-item";

    if (selectedChatId === parseInt(id, 10)) {
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
          <div className="chat-preview">{chat.preview}</div>
        </div>
      </div>
    );
  };

  isRowLoaded = ({ index }) => {
    !!this.props.chats[index];
  };

  loadMoreRows = ({ startIndex, stopIndex }) => {
    this.props.loadChatList(startIndex, stopIndex);
  };

  render() {
    let { selectedAccount, chats, chatLength } = this.props;

    if (selectedAccount == null) {
      return <div>Please login</div>;
    }

    return (
      <AutoSizer disableWidth>
        {({ height }) => (
          <InfiniteLoader
            isRowLoaded={this.isRowLoaded}
            loadMoreRows={this.loadMoreRows}
            rowCount={chatLength}
          >
            {({ onRowsRendered, registerChild }) => (
              <List
                className="chat-list"
                height={height}
                onRowsRendered={onRowsRendered}
                ref={registerChild}
                rowCount={chatLength}
                rowHeight={80}
                rowRenderer={this.rowRenderer}
                width={350}
                {
                  ...this.props /* Force rerender when props change*/
                }
              />
            )}
          </InfiniteLoader>
        )}
      </AutoSizer>
    );
  }
}

const mapStateToProps = state => {
  let {
    selected_account,
    selected_chat,
    selected_chat_id,
    chats,
    selected_chat_length
  } = state.shared;

  return {
    chats,
    selectedAccount: selected_account,
    selectedChat: selected_chat,
    selectedChatId: selected_chat_id,
    chatLength: selected_chat_length || 0
  };
};

const mapDispatchToProps = {
  selectChat,
  loadChatList
};

export default connect(mapStateToProps, mapDispatchToProps)(ChatList);
