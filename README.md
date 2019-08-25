[![Travis CI Build Status](https://travis-ci.org/Y2Z/monolith.svg?branch=master)](https://travis-ci.org/Y2Z/monolith)
[![AppVeyor Build status](https://ci.appveyor.com/api/projects/status/j1v1d96sw952b1ch?svg=true)](https://ci.appveyor.com/project/vflyson/monolith)

# monolith

A data hoarder's dream come true: bundle any web page into a single HTML file.  
You can finally replace that gazillion of open tabs with a gazillion of .html files stored somewhere on your precious little drive.

Unlike the conventional "Save page as", `monolith` not only saves the target document, it embeds CSS, image, and JavaScript assets **all at once**, producing a single HTML5 document that is a joy to store and share.

If compared to saving websites with `wget -mpk`, this tool embeds all assets as data URLs and therefore lets browsers render the saved page exactly the way it was on the Internet, even when no network connection is available.

<!-- `This program works both on remote and local targets. -->

### Installation
    $ git clone https://github.com/Y2Z/monolith.git
    $ cd monolith
    $ cargo install --path .

### Usage
    $ monolith https://lyrics.github.io/db/p/portishead/dummy/roads/ > portishead-roads-lyrics.html

### Options
 - `-i`: Remove images
 - `-j`: Exclude JavaScript
 - `-s`: Silent mode
 - `-u`: Specify custom User-Agent

### License
The Unlicense

<!-- Microtext -->
<sub>Keep in mind that `monolith` is not aware of your browser's session</sub>
