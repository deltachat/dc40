import "modern-css-reset";

import "./src/style.less";

import("./pkg").then(module => {
  module.run_app();
});
