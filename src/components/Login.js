import React from 'react'
import { connect } from 'react-redux'
import Files from 'react-files'
import { login, imex } from '../redux/index'

class Login extends React.Component {
  constructor(props) {
    super(props)

    this.files = React.createRef()

    this.state = {
      email: '',
      password: '',
      path: null
    }
  }
  
  onEmailChange = (event) => {
    this.setState({ email: event.target.value })
  }

  onPasswordChange = (event) => {
    this.setState({ password: event.target.value })
  }

  onCancel = (event) => {
    event.preventDefault()
    this.props.onCancel && this.props.onCancel()
  }
  
  onSubmit = (event) => {
    event.preventDefault()

    if (this.state.path != null) {
      this.props.imex(this.state.email, this.state.path)
    } else {
      this.props.login(this.state.email, this.state.password)
    }

    this.files.current.removeFiles()
    
    this.setState ({
      password: '',
      path: null
    })

    this.props.onSubmit && this.props.onSubmit()
  }

  onFilesChange = (files) => {
    if (files && files.length > 0) {
      this.setState({ path: files[0].path })
    }
  }

  render() {
    return (
        <div className="login">
          <h4>Add Account</h4>
        <form onSubmit={this.onSubmit}>
          <label>Email:</label>
            <input type="text" value={this.state.email} onChange={this.onEmailChange} />
            <label>Password:</label>
        <input type="password" value={this.state.password} onChange={this.onPasswordChange} />
        <Files
      ref={this.files}
      multiple={false}
      maxFiles={1}
      accepts={['.bak']}
      onChange={this.onFilesChange}
      clickable
        >
        Import Backup: {this.state.path}
        </Files>
      
        <input type="submit" value={this.state.path != null ? "Import" : "Login" } />
        <input type="button" value="Cancel" onClick={this.onCancel} />
        </form>
        </div>
    )
  }
}

Login.defaultProps = {
};

const mapStateToProps = (state) => ({});

const mapDispatchToProps = {
  login,
  imex
};

export default connect(mapStateToProps, mapDispatchToProps)(Login);
