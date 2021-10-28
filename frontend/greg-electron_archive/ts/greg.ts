import 'bulma/css/bulma.css';
import '@fortawesome/fontawesome-free/css/fontawesome.css';
import '@fortawesome/fontawesome-free/js/all.js';

import '../less/greg.less';

document.addEventListener('DOMContentLoaded', function() {
});

var K_UP = 38;
var K_DOWN = 40;
var K_RIGHT = 39;
var K_LEFT = 37;
var K_L = 76;
var K_K = 75;
var K_ESC = 27;

document.onkeydown = function (eventObject) {
    if (eventObject.keyCode == K_L && eventObject.ctrlKey) {
        console.log("Focusing url");
        var url = document.getElementById("url");
        url.focus();
        if (eventObject.shiftKey) {
            var pas = document.getElementById("private-address-sections");
            if (pas.style.display == "none") {
                pas.style.display = "block";
            } else {
                pas.style.display = "none";
            }
        }
    }
    if (eventObject.keyCode == K_L && eventObject.ctrlKey) {
        console.log("Focusing url");
        var url = document.getElementById("url");
        url.focus();
    }
    if (eventObject.keyCode == K_K && eventObject.ctrlKey) {
        console.log("Focusing clipboard");
    }
    if (eventObject.keyCode == K_ESC && document.activeElement.className.split(" ").indexOf("address-entry") > -1){
        var main = document.getElementById("main");
        console.log(main);
        main.scrollIntoView();
        var center = document.getElementById("center");
        center.focus();
    }
    if (eventObject.keyCode == K_UP) {
        var focused = document.getElementsByClassName("focused");
        focused[0].scrollIntoView();
    }
};

