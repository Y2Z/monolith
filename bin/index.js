#!/usr/bin/env nodejs

var compactor = require('../compactor.js');
var options = require('../options.js');

function printUsage () {
    console.log("\nUsage: \n       monolith https://github.com\n")
}

if (process.argv.length > 2) {
    var target = null

    for (var i = 2, ilen = process.argv.length; i < ilen; i++) {
        var argument = process.argv[i]

        if (argument == '--data-uri' || argument == '-u') {
            options.outputFinalResultAsBase64 = true
        } else if (argument == '--quiet' || argument == '-q') {
            options.suppressVerboseOutput = true
        } else {
            if (!target) {
                target = argument
            } else {
                // Can't have more than one target
                target = null
                break
            }
        }
    }

    if (target) {
        compactor(target, function(error, result) {
            console.log(result)
        })
    } else {
        printUsage()
    }
} else {
    printUsage()
}
