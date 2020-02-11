import React from "react";
import { connect } from "react-redux";
import ReactModal from "react-modal";
import { Icon } from "react-icons-kit";
import { plusSmall } from "react-icons-kit/oct/plusSmall";

import { selectAccount } from "../redux";
import Login from "./login";

class AccountList extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      email: "",
      password: "",
      showAddAccount: false
    };
  }

  handleOpenModal = () => {
    this.setState({ showAddAccount: true });
  };

  handleCloseModal = () => {
    this.setState({ showAddAccount: false });
  };

  onAccountClick(email, event) {
    event.preventDefault();
    this.props.selectAccount(this.props.selected_account, email);
  }

  render() {
    let { accounts, selected_account } = this.props;

    return (
      <div className="account-list">
        {accounts.map(account => (
          <div
            className="account"
            key={account.email}
            onClick={this.onAccountClick.bind(this, account.email)}
          >
            <div className="letter-icon">{account.email[0]}</div>
          </div>
        ))}

        <a
          className="account button"
          onClick={this.handleOpenModal}
          alt="Add Account"
        >
          <Icon icon={plusSmall} size={32} />
        </a>

        <ReactModal
          isOpen={this.state.showAddAccount}
          contentLabel="Add Account"
        >
          <Login
            onSubmit={this.handleCloseModal}
            onCancel={this.handleCloseModal}
          />
        </ReactModal>
      </div>
    );
  }
}

const mapStateToProps = state => ({
  accounts: Object.values(state.shared.accounts),
  selected_account: state.shared.selected_account
});

const mapDispatchToProps = {
  selectAccount
};

export default connect(mapStateToProps, mapDispatchToProps)(AccountList);
