const electron = require("electron");
const app = electron.app;
const protocol = electron.protocol;

const BrowserWindow = electron.BrowserWindow;

const path = require("path");
const isDev = require("electron-is-dev")
isDev && require('react-devtools-electron')

let mainWindow;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1024,
    height: 786
  });

  mainWindow.loadURL(
    isDev
      ? "http://localhost:3000"
      : `file://${path.join(__dirname, "../build/index.html")}`
  );
  mainWindow.on("closed", () => (mainWindow = null));

  protocol.registerFileProtocol('dc', (request, callback) => {
    const url = request.url.substr(4)
    // TODO: secure to be only able to read from .deltachat folder
    callback(({ path: url }))
  }, (error) => {
    if (error) {
      console.error('Failed to register protocol')
    }
  })
}

app.on("ready", createWindow);

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", () => {
  if (mainWindow === null) {
    createWindow();
  }
});
