'use strict';

var functions    = require('../functions.js')
var retrieveFile = functions.retrieveFile

var dataURI = true // set to true to convert the src attribute to a dataURI link

module.exports = {

    parser: function (window, absBasePath) {
        var scripts = window.document.getElementsByTagName('script')

        for (var i = 0, ilen = scripts.length; i < ilen; i++) {
            if (scripts[i].getAttribute('src')) {
                var data = retrieveFile(absBasePath, scripts[i].getAttribute('src').trim(), dataURI)

                if (dataURI) {
                    scripts[i].setAttribute('src', "data:text/javascript;base64," + data)
                } else {
                    scripts[i].removeAttribute('src')
                    scripts[i].innerHTML = data
                }
            }
        }
    }

}
