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
import moment from "moment";

import { loadMessageList } from "../redux";

class Chat extends React.Component {
  constructor(props) {
    super(props);

    this._cache = new CellMeasurerCache({
      fixedWidth: true,
      defaultHeight: 50,
      minHeight: 30
    });

    this.state = {
      email: "",
      password: "",
      scrollTop: props.messagesLength - 1
    };

    this.list = null;
  }

  componentDidUpdate(prevProps, prevState, snapshot) {
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
    const { messages } = this.props;
    const msg = messages[index];

    if (msg == null) {
      return <div key={key} style={style}></div>;
    }

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
            <div className="message-text">{msg.text || msg.viewtype}</div>
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

  isRowLoaded = ({ index }) => {
    !!this.props.messages[index];
  };

  loadMoreRows = ({ startIndex, stopIndex }) => {
    this.props.loadMessageList(startIndex, stopIndex);
  };

  render() {
    let { messages, messagesLength, selectedChat } = this.props;
    let { scrollTop } = this.state;

    if (messages == null || selectedChat == null) {
      return <div>Please select a chat</div>;
    }

    return (
      <div className="chat">
        <WindowSizeListener onResize={this.onResize} />
        <div className="chat-header">{selectedChat.name}</div>
        <div className="message-list">
          <AutoSizer>
            {({ width, height }) => (
              <InfiniteLoader
                isRowLoaded={this.isRowLoaded}
                loadMoreRows={this.loadMoreRows}
                rowCount={messagesLength}
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
                    {
                      ...this.props /* Force rerender when props change*/
                    }
                  />
                )}
              </InfiniteLoader>
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
  let { selected_chat, messages, selected_messages_length } = state.shared;

  return {
    selectedChat: selected_chat,
    messages,
    messagesLength: selected_messages_length || 0
  };
};

const mapDispatchToProps = {
  loadMessageList
};

export default connect(mapStateToProps, mapDispatchToProps)(Chat);
