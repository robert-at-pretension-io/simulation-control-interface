import("../pkg/index.js").catch(console.error);

import { u } from "umbrellajs";


// Might as well give credit...
// https://stackoverflow.com/questions/41174095/do-i-need-to-use-onload-to-start-my-webpack-bundled-code
// Rather give credit than find and read the requisit documentation...
// This is acceptable because the heart of the application is in the rust code... Not in the frontend
function ready(fn) {
    if (document.readyState != 'loading'){
      fn();
    } else {
      document.addEventListener('DOMContentLoaded', fn);
    }
  }
  
  ready(function() {




u('div#app').after("<p> App Contents </p>");

u("div#nav").after("<p> Nav Bar </p>");

u("div#message_bar").after("<p> Message Bar </p>");

u("div#app").on("new_message", (e) => {
  console.log("Received the following event details: " + JSON.stringify(e) + JSON.stringify(e.detail));
});

u("<body>").on("onload", console.log("page loaded"));


})
  