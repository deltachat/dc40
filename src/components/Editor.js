import React from "react";
import Editor from "draft-js-plugins-editor";
import { EditorState, ContentState } from "draft-js";
import createEmojiMartPlugin from "draft-js-emoji-mart-plugin";
import data from "emoji-mart/data/apple.json";
import { Emoji } from "emoji-mart";
import { Icon } from "react-icons-kit";
import { androidHappy } from "react-icons-kit/ionicons/androidHappy";
import { upload } from "react-icons-kit/ionicons/upload";
import { getDefaultKeyBinding, KeyBindingUtil } from "draft-js";
import Files from "react-files";

import "emoji-mart/css/emoji-mart.css";

const emojiPlugin = createEmojiMartPlugin({
  data,
  set: "apple",
  emojiSize: 32,
  sheetSize: 64
});
const { Picker } = emojiPlugin;

function myKeyBindingFn(e) {
  if (e.keyCode === 13 /* `Enter` key */) {
    return "myeditor-send";
  }
  return getDefaultKeyBinding(e);
}

// customized editor with plugins
class MyEditor extends React.Component {
  constructor(props) {
    super(props);

    this.files = React.createRef();
    this.state = {
      showPicker: false,
      editorState: EditorState.createEmpty()
    };
  }

  componentDidMount() {
    this.focusEditor();
  }

  setEditor = editor => {
    this.editor = editor;
  };

  focusEditor = () => {
    if (this.editor) {
      this.editor.focus();
    }
  };

  getTextAndRemove() {
    const text = this.state.editorState.getCurrentContent().getPlainText("\n");

    // clear text
    const editorState = EditorState.push(
      this.state.editorState,
      ContentState.createFromText("")
    );
    this.setState({ editorState });

    return text;
  }

  handleKeyCommand = command => {
    if (command === "myeditor-send") {
      const text = this.getTextAndRemove();
      this.props.onEnter && this.props.onEnter(text);

      return "handled";
    }

    return "not-handled";
  };

  onPickerButtonClick = () => {
    this.setState({ showPicker: true });
  };

  onEmojiSelect = () => {
    this.setState({ showPicker: true });
  };

  onChange = editorState => {
    this.setState({ editorState, showPicker: false });
  };

  onFilesChange = files => {
    if (files && files.length > 0) {
      const text = this.getTextAndRemove();
      this.props.onEnter && this.props.onFile(files[0], text);
      this.files.current.removeFiles();
    }
  };

  render() {
    const { editorState, onChange } = this.props;
    const { showPicker } = this.state;

    return (
      <div className="chat-input" onClick={this.focusEditor}>
        <div className="chat-editor">
          <Editor
            editorState={this.state.editorState}
            handleKeyCommand={this.handleKeyCommand}
            keyBindingFn={myKeyBindingFn}
            onChange={this.onChange}
            plugins={[emojiPlugin]}
          />
        </div>
        <Files
          ref={this.files}
          multiple={false}
          maxFiles={1}
          onChange={this.onFilesChange}
          clickable
        >
          <div className="chat-files-picker">
            <Icon icon={upload} size={32} />
          </div>
        </Files>
        {showPicker ? (
          <Picker
            className="chat-emoji-picker"
            perLine={7}
            showPreview={false}
          />
        ) : (
          <div className="chat-emoji-picker-button">
            <Icon
              icon={androidHappy}
              size={32}
              onClick={this.onPickerButtonClick}
            />
          </div>
        )}
      </div>
    );
  }
}

export default MyEditor;
