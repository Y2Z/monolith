'use strict'

var functions    = require('../functions.js')
var retrieveFile = functions.retrieveFile
var mime         = require('./img.js').mime

//var reIcon = /^([a-z]+\s)?icon(\s[a-z]+)?$/i
var reIcon = /icon/i

module.exports = {

    parser: function (window, absBasePath) {
        var links = window.document.head.getElementsByTagName('link')

        for (var i = 0, ilen = links.length; i < ilen; i++) {
            if (reIcon.test(links[i].getAttribute('rel'))) {
                var data = retrieveFile(absBasePath, links[i].getAttribute('href').trim(), true)

                links[i].setAttribute('href', "data:" + mime(data) + "base64," + data)
            }
        }
    }

}
