var fs      = require('fs')
var path    = require('path')
var url     = require('url')
var request = require('sync-request')

var options = require('./options.js');

var cache = {}

var request_options = {
    'headers': {
        'user-agent': options.userAgentString
    }
}

// Note: http://site.com/image/icons/home.png -> http://site.com/image/icons
function absoluteURLPath (aURL) {
    var URL = url.parse(aURL)

    return URL.protocol + '//' + URL.host + URL.path
}

var reURL = /^https?:\/\//i // TODO file:///

function isURL (aPath) { return reURL.test(aPath) }
function base64 (aInput) { return new Buffer(aInput).toString('base64') }

function resolve (aFrom, aTo) {
    if (isURL(aFrom)) {
        var URL = url.parse(aFrom)

        if (aTo[0] == '/') { // (http://site.com/article/1, /css/main.css)
            if (aTo[1] == '/') { // (http://site.com/article/1, //images/1.png)
                return URL.protocol + aTo
            } else {
                return url.resolve(URL.protocol + '//' + URL.host, aTo)
            }
        } else if (isURL(aTo)) { // (http://site.com, http://site.com/css/main.css)
            return aTo
        } else { // (http://site.com, css/main.css)
            return url.resolve(aFrom, aTo)
        }
    } else {
        return path.resolve(aFrom, aTo)
    }
}

function retrieveFile (aAbsBasePath, aFilePath, aBinary) {
    var fullFilePath = resolve(aAbsBasePath, aFilePath)
    var format = aBinary ? 'base64' : 'utf8'
    var cacheKey = fullFilePath + '@' + format

    if (isURL(fullFilePath)) {
        if (cacheKey in cache) {
            return cache[cacheKey]
        } else {
            try {
                var res = request('GET', fullFilePath, request_options)

                if (!options.suppressVerboseOutput)
                    console.warn('Retrieving file', fullFilePath, '...')

                return cache[cacheKey] = res.getBody(format)
            } catch (httpError) {
                return ''
            }
        }
    } else {
        return fs.readFileSync(fullFilePath, format)
    }
}

module.exports = { absoluteURLPath, isURL, base64, resolve, retrieveFile }
