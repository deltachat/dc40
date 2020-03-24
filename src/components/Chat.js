import React from "react";
import { connect } from "react-redux";
import {
  List,
  InfiniteLoader,
  AutoSizer,
  CellMeasurer,
  CellMeasurerCache
} from "react-virtualized";
import WindowSizeListener from "react-window-size-listener";
import ReactTextFormat from "react-text-format";
import moment from "moment";
import { isEqual } from "lodash";

import {
  loadMessageList,
  sendTextMessage,
  sendFileMessage,
  createChatById
} from "../redux";
import Editor from "./Editor";

class Chat extends React.Component {
  constructor(props) {
    super(props);

    this._cache = new CellMeasurerCache({
      fixedWidth: true,
      defaultHeight: 60,
      minHeight: 25
    });

    this.state = {
      email: "",
      password: ""
    };

    this.infiniteLoader = React.createRef();
  }

  componentDidUpdate(prevProps, prevState, snapshot) {
    if (this.props.selectedChatId != prevProps.selectedChatId) {
      // clear cache when we change the chat
      this._cache.clearAll();
      this.loadMoreRows({
        startIndex: this.props.messagesLength - 20,
        stopIndex: this.props.messagesLength
      });
      this.infiniteLoader.current &&
        this.infiniteLoader.current.resetLoadMoreRowsCache();
    }

    return null;
  }

  onResize = () => {
    this._cache.clearAll();
  };

  rowRenderer = ({ index, isScrolling, isVisible, key, parent, style }) => {
    const { messages } = this.props;
    const msg = messages[index];

    if (msg == null) {
      return <div key={key} style={style}></div>;
    }

    let messageClassName = "message";

    let content;

    // check if we have a day switch
    let newDay = null;
    if (index > 0 && messages[index - 1] != null) {
      let a = moment.unix(msg.timestamp);
      let b = moment.unix(messages[index - 1].timestamp);
      if (a.isAfter(b, "day")) {
        let date = a.calendar(null, {
          sameDay: "[Today]",
          nextDay: "[Tomorrow]",
          nextWeek: "dddd, MMMM, do",
          lastDay: "[Yesterday]",
          lastWeek: "dddd, MMMM, do",
          sameElse: "dddd, MMMM, do"
        });
        newDay = <div className="message-new-day">{date}</div>;
      }
    }

    if (msg.is_info) {
      content = <div className="message-info">{msg.text}</div>;
    } else {
      // was the previous message from the same as this message?
      let sameSender = false;
      if (index > 0 && messages[index - 1] != null) {
        sameSender = messages[index - 1].from_id === msg.from_id;
      }

      const imageStyle = {
        backgroundColor: "#" + msg.from_color.toString(16)
      };

      let image;
      let header;

      if (!sameSender) {
        messageClassName += " first";

        if (msg.from_profile_image != null) {
          image = (
            <img
              className="image-icon"
              src={"dc://" + msg.from_profile_image}
              alt="avatar"
            />
          );
        } else {
          image = (
            <div className="letter-icon" style={imageStyle}>
              {msg.from_first_name[0]}
            </div>
          );
        }
        header = (
          <div className="message-header">
            <div className="message-sender">{msg.from_first_name}</div>
            <div className="message-timestamp">
              {moment.unix(msg.timestamp).format("h:mm")}
            </div>
          </div>
        );
      }

      let file = null;
      if (
        msg.file != null &&
        (msg.viewtype === "Image" || msg.viewtype === "Gif")
      ) {
        let height = Math.min(msg.file_height, 300);
        let width = "auto";

        file = (
          <div className="message-image">
            <img
              src={"dc://" + msg.file}
              alt="image"
              height={height}
              width={width}
            />
          </div>
        );
      }

      content = (
        <div className="message-text">
          <div className="message-icon">{image}</div>
          <div className="message-body">
            {header}
            <div className="message-inner-text">
              <ReactTextFormat
                linkTarget="_blank"
                allowedFormats={[
                  "URL",
                  "Email",
                  "Image",
                  "Phone",
                  "CreditCard"
                ]}
              >
                {msg.text}
              </ReactTextFormat>
            </div>
            {file}
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
        <div className={messageClassName} style={style}>
          {newDay}
          <div className="message-menu">
            <button onClick={this.onChatClick.bind(this, msg.id)}>Chat</button>
          </div>
          <div className="message-content">{content}</div>
        </div>
      </CellMeasurer>
    );
  };

  onChatClick(id) {
    event.preventDefault();
    console.log("Chat", id);
    this.props.createChatById(id);
  }

  isRowLoaded = ({ index }) => {
    !!this.props.messages[index];
  };

  loadMoreRows = ({ startIndex, stopIndex }) => {
    this.props.loadMessageList(startIndex, stopIndex);
  };

  onSendTextMessage = text => {
    this.props.sendTextMessage(text);
  };

  onSendFileMessage = (file, text) => {
    const path = file.path;
    const mime = file.type; // mime type
    let typ = "File";
    switch (file.extension) {
      case "png":
      case "jpg":
      case "jpeg":
      case "webp":
      case "tiff":
      case "raw":
        // image
        typ = "Image";
        break;
      case "gif":
        typ = "Gif";
        break;
      default:
        break;
    }

    // TODO: detect more formats

    this.props.sendFileMessage(typ, path, text, mime);
  };

  render() {
    let { messages, messagesLength, selectedChat, selectedChatId } = this.props;

    if (messages == null || selectedChat == null) {
      return <div>Please select a chat</div>;
    }

    return (
      <div className="chat">
        <WindowSizeListener onResize={this.onResize} />
        <div className="chat-header">
          <div className="chat-header-name">{selectedChat.name}</div>
          <div className="chat-header-subtitle">{selectedChat.subtitle}</div>
        </div>

        <div className="message-list">
          <AutoSizer>
            {({ width, height }) => (
              <InfiniteLoader
                ref={this.infiniteLoader}
                isRowLoaded={this.isRowLoaded}
                loadMoreRows={this.loadMoreRows}
                rowCount={messagesLength}
                selectedChat={selectedChat}
              >
                {({ onRowsRendered, registerChild }) => (
                  <List
                    height={height}
                    rowCount={messagesLength}
                    rowHeight={this._cache.rowHeight}
                    rowRenderer={this.rowRenderer}
                    width={width - 10}
                    deferredMeasurementCache={this._cache}
                    ref={registerChild}
                    scrollToIndex={messagesLength}
                    onRowsRendered={onRowsRendered}
                    selectedChat={selectedChat}
                  />
                )}
              </InfiniteLoader>
            )}
          </AutoSizer>
        </div>
        <Editor
          placeholder="Type message"
          onEnter={this.onSendTextMessage}
          onFile={this.onSendFileMessage}
        />
      </div>
    );
  }
}

const mapStateToProps = state => {
  let {
    selected_chat,
    selected_chat_id,
    messages,
    selected_messages_length
  } = state.shared;

  return {
    selectedChatId: selected_chat_id,
    selectedChat: selected_chat,
    messages,
    messagesLength: selected_messages_length || 0
  };
};

const mapDispatchToProps = {
  loadMessageList,
  sendTextMessage,
  sendFileMessage,
  createChatById
};

export default connect(mapStateToProps, mapDispatchToProps, null, {
  areStatePropsEqual: isEqual
})(Chat);
