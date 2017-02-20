'use strict';

var functions = require('../functions.js')

module.exports = {

    parser: function (window, absBasePath) {
        var anchors = window.document.getElementsByTagName('a')

        for (var i = 0, ilen = anchors.length; i < ilen; i++) {
            if (anchors[i].getAttribute('href')) {
                var anchor = anchors[i]
                var href = anchor.getAttribute('href').trim()
                var absoluteURL = functions.resolve(absBasePath, href)

                anchor.setAttribute('href', absoluteURL)
            }
        }
    }

}
