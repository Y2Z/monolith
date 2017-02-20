'use strict'

var path  = require('path')
var jsdom = require('jsdom')

var options         = require('./options.js');
var functions       = require('./functions.js')
var absoluteURLPath = functions.absoluteURLPath,
isURL               = functions.isURL,
base64              = functions.base64,
resolve             = functions.resolve,
retrieveFile        = functions.retrieveFile

var modules = [
    // 1. CSS
    require('./modules/css.js').parser,
    // 2. JS
    require('./modules/js.js').parser,
    // 3. images
    require('./modules/img.js').parser,
    // 4. favicon
    require('./modules/favicon.js').parser,
    // 5. anchors
    require('./modules/anchors.js').parser,
]

function monolith (targetDocumentPath, _options, callback) {
    // In case monolith got included as a library
    for (var k in _options)
        options[k] = _options[k]

    // Determine the absolute initial document path
    var absBasePath = isURL(targetDocumentPath)
                      ? absoluteURLPath(targetDocumentPath)
                      : path.dirname(path.resolve(targetDocumentPath))
    absBasePath += '/' // Append trailing slash

    // Retrieve the root document to use as a base
    var rootFileContent = retrieveFile(absBasePath, targetDocumentPath)

    // Convert the target document into a DOM tree
    jsdom.env(rootFileContent, [], function (err, window) {
        for (var i = 0, ilen = modules.length; i < ilen; i++)
            modules[i](window, absBasePath)

        var result = window.document.documentElement.innerHTML
        callback(null, options.outputFinalResultAsBase64 ? base64(result) : result)
    })
}

module.exports = monolith
