'use strict';

var functions = require('../functions.js')

module.exports = {

    parser: function (window, absBasePath) {
        var anchors = window.document.getElementsByTagName('a')

        for (var i = 0, ilen = anchors.length; i < ilen; i++) {
            if (anchors[i].getAttribute('href')) {
                var anchor = anchors[i]
                var href = anchor.getAttribute('href').trim()

                // Do not touch hrefs which start with a pound sign
                if (href[0] == '#') continue

                var absoluteURL = functions.resolve(absBasePath, href)

                anchor.setAttribute('href', absoluteURL)
            }
        }
    }

}
