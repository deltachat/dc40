import React from 'react';
import { connect } from 'react-redux';
import { say } from '../redux/index';

class Messages extends React.Component {
    render() {
      const { details, say } = this.props;
        return (
            <div className="Messages">
                <ul className="Details--list">
                    { Object.entries(details).map((message, index) => (
                        <li key={index}>{message}</li>
                    ))}
                </ul>
                <form onSubmit={(e) => {
                    e.preventDefault();
                    say(this.messageInput.value);
                    this.messageInput.value = '';
                }}>
                    <input type="text" ref={ref => this.messageInput = ref}/>
                    <button>Say</button>
                </form>
            </div>
        );
    }
}

Messages.defaultProps = {
};

const mapStateToProps = (state) => ({
  details: state.shared.details,
});

const mapDispatchToProps = {
    say,
};

export default connect(mapStateToProps, mapDispatchToProps)(Messages);
