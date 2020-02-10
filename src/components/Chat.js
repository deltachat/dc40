import React from "react";
import { connect } from "react-redux";
import {
  List,
  AutoSizer,
  CellMeasurer,
  CellMeasurerCache
} from "react-virtualized";
import WindowSizeListener from "react-window-size-listener";
import moment from "moment";

class Chat extends React.Component {
  constructor(props) {
    super(props);

    this._cache = new CellMeasurerCache({
      defaultWidth: 400,
      defaultHeight: 50,
      minWidth: 100,
      minHeight: 30
    });

    this.state = {
      email: "",
      password: ""
    };

    this.list = React.createRef();
  }

  componentDidUpdate(prevProps, prevState, snapshot) {
    const prev_chats = prevProps.selectedChatMsgs;
    const curr_chats = this.props.selectedChatMsgs;

    if (!prev_chats || (curr_chats && prev_chats.length != curr_chats)) {
      // scroll to the end on initial render, and when the length changes
      this.list.current && this.list.current.scrollToRow(curr_chats.length - 1);
    }

    if (this.props.selectedChat != prevProps.selectedChat) {
      // clear cache when we change the chat
      this._cache.clearAll();
    }

    return null;
  }

  onResize = () => {
    this._cache.clearAll();
  };

  rowRenderer = ({ index, isScrolling, isVisible, key, parent, style }) => {
    const { selectedChat, selectedChatMsgs } = this.props;

    if (selectedChat == null || selectedChatMsgs == null) {
      return <div key={key} style={style}></div>;
    }

    const msg = Object.values(selectedChatMsgs)[index];
    // TODO: handle non text messages

    let content;

    if (msg.is_info) {
      content = <div className="message-info">{msg.text}</div>;
    } else {
      const imageStyle = {
        backgroundColor: "#" + msg.from_color.toString(16)
      };

      let image = (
        <div className="letter-icon" style={imageStyle}>
          {msg.from_first_name[0]}
        </div>
      );
      if (msg.from_profile_image != null) {
        image = (
          <img
            className="image-icon"
            src={"dc://" + msg.from_profile_image}
            alt="avatar"
          />
        );
      }
      content = (
        <div className="message-text">
          <div className="message-icon">{image}</div>
          <div className="message-body">
            <div className="message-header">
              <div className="message-sender">{msg.from_first_name}</div>
              <div className="message-timestamp">
                {moment.unix(msg.timestamp).format("h:mm")}
              </div>
            </div>
            <div className="message-text">{msg.text}</div>
          </div>
        </div>
      );
    }

    return (
      <CellMeasurer
        cache={this._cache}
        columnIndex={0}
        key={key}
        parent={parent}
        rowIndex={index}
      >
        <div className="message" style={style}>
          {content}
        </div>
      </CellMeasurer>
    );
  };

  render() {
    let { selectedChat, selectedChatMsgs } = this.props;

    if (selectedChat == null) {
      return <div>Please select a chat</div>;
    }

    let rowCount =
      selectedChatMsgs != null ? Object.keys(selectedChatMsgs).length : 0;

    return (
      <div className="chat">
        <WindowSizeListener onResize={this.onResize} />
        <div className="chat-header">{selectedChat.name}</div>
        <div className="message-list">
          <AutoSizer>
            {({ width, height }) => (
              <List
                height={height}
                rowCount={rowCount}
                rowHeight={this._cache.rowHeight}
                rowRenderer={this.rowRenderer}
                width={width - 10}
                deferredMeasurementCache={this._cache}
                ref={this.list}
                {
                  ...this.props /* Force rerender when props change*/
                }
              />
            )}
          </AutoSizer>
        </div>
        <div className="chat-input">
          <input type="text" />
        </div>
      </div>
    );
  }
}

const mapStateToProps = state => {
  let selected = state.shared.selected_account;
  let accounts = state.shared.accounts;
  let account = selected != null && accounts != null && accounts[selected];

  return {
    selectedAccount: selected,
    selectedChatId: account && account.selected_chat,
    selectedChat:
      account && account.selected_chat && account.chats[account.selected_chat],
    selectedChatMsgs: account && account.selected_chat_msgs
  };
};

const mapDispatchToProps = {};

export default connect(mapStateToProps, mapDispatchToProps)(Chat);
