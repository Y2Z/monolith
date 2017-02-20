'use strict';

var functions    = require('../functions.js')
var retrieveFile = functions.retrieveFile

module.exports = {

    parser: function (window, absBasePath) {
        var links = window.document.head.getElementsByTagName('link')

        for (var i = 0, ilen = links.length; i < ilen; i++) {
            if (links[i].getAttribute('rel') == 'stylesheet') {
                var data = retrieveFile(absBasePath, links[i].getAttribute('href').trim(), true)

                links[i].setAttribute('href', "data:text/css;base64," + data)
            }
        }
    }

}
