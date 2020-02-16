const electron = require("electron");
const app = electron.app;
const protocol = electron.protocol;
const shell = electron.shell;
const ipc = electron.ipcMain;

const BrowserWindow = electron.BrowserWindow;

const path = require("path");
const isDev = require("electron-is-dev");
isDev && require("react-devtools-electron");

let mainWindow;

const gotTheLock = app.requestSingleInstanceLock();

function start() {
  if (!gotTheLock) {
    app.quit();
    n;
    return;
  }

  app.on("second-instance", (event, commandLine, workingDirectory) => {
    // Someone tried to run a second instance, we should focus our window.
    if (mainWindow) {
      if (mainWindow.isMinimized()) {
        myWindow.restore();
      }

      mainWindow.focus();
    }
  });
  app.on("ready", createWindow);
}

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

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1024,
    height: 786,
    frame: false,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      enableRemoteModule: false,
      preload: path.join(app.getAppPath(), "src/preload.js")
    }
  });

  mainWindow.loadURL(
    isDev
      ? "http://localhost:3000"
      : `file://${path.join(__dirname, "../build/index.html")}`
  );
  mainWindow.on("closed", () => (mainWindow = null));

  protocol.registerFileProtocol(
    "dc",
    (request, callback) => {
      const url = request.url.substr(4);
      // TODO: secure to be only able to read from .deltachat folder
      callback({ path: url });
    },
    error => {
      if (error) {
        console.error("Failed to register protocol");
      }
    }
  );

  mainWindow.webContents.on("new-window", (event, url) => {
    event.preventDefault();
    console.log("opening", url);
    shell.openExternal(url);
  });

  ipc.on("toMain", function(arg, event) {
    switch (event) {
      case "minimize":
        mainWindow.minimize();
        break;
      case "maximize":
        mainWindow.maximize();
        break;
      case "close":
        app.quit();
        break;
      case "enter-full-screen":
        mainWindow.setFullScreen(true);
        break;
      case "exit-full-screen":
        mainWindow.setFullScreen(false);
        break;
      default:
        break;
    }
  });
}

start();
