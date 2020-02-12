import React from "react";
import { connect } from "react-redux";
import ReactModal from "react-modal";
import { Icon } from "react-icons-kit";
import { plusSmall } from "react-icons-kit/oct/plusSmall";
import { isEqual } from "lodash";

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
    let { accounts, selectedAccount } = this.props;

    let accountList = accounts.map(account => {
      let className = "account";
      if (account.email === selectedAccount) {
        className += " active";
      }

      return (
        <div
          className={className}
          key={account.email}
          onClick={this.onAccountClick.bind(this, account.email)}
        >
          <div className="letter-icon">{account.email[0]}</div>
        </div>
      );
    });

    return (
      <div className="account-list">
        {accountList}

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
  selectedAccount: state.shared.selected_account
});

const mapDispatchToProps = {
  selectAccount
};

export default connect(mapStateToProps, mapDispatchToProps, null, {
  areStatePropsEqual: isEqual
})(AccountList);
