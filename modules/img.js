'use strict';

var functions    = require('../functions.js')
var retrieveFile = functions.retrieveFile

function mime (data) {
    var mime = 'image/jpeg'

    if (~data.indexOf('iVBORw0K'))
        mime = 'image/png'
    else if (~data.indexOf('R0lGODlh'))
        mime = 'image/gif'
    else if (~data.indexOf('<?xml'))
        mime = 'image/svg+xml'

    return mime
}

module.exports = {

    mime: mime,

    parser: function (window, absBasePath) {
        // <img>, <picture> <img>
        var imgs = window.document.getElementsByTagName('img')

        for (var i = 0, ilen = imgs.length; i < ilen; i++) {
            var img = imgs[i]

            if (img.getAttribute('src')) {
                var data = retrieveFile(absBasePath, img.getAttribute('src').trim(), true)

                img.setAttribute('src', "data:" + mime(data) + ";base64," + data)
            }
        }

        // <picture> <source>
        var pictures = window.document.getElementsByTagName('picture')

        for (var i = 0, ilen = pictures.length; i < ilen; i++) {
            var picture = pictures[i]
            var sources = picture.getElementsByTagName('source')

            for (var s = 0, slen = sources.length; s < slen; s++) {
                var source = sources[s]

                if (source.getAttribute('srcset')) {
                    var data = retrieveFile(absBasePath, source.getAttribute('srcset').trim(), true)
                    var type = source.getAttribute('type')

                    source.setAttribute('srcset', "data:" + (type || mime(data)) + ";base64," + data)
                }
            }
        }
    }

}
