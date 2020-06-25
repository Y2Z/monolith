[![Monolith Build Status for GNU/Linux](https://github.com/Y2Z/monolith/workflows/GNU%2FLinux/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AGNU%2FLinux)
[![Monolith Build Status for macOS](https://github.com/Y2Z/monolith/workflows/macOS/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AmacOS)
[![Monolith Build Status for Windows](https://github.com/Y2Z/monolith/workflows/Windows/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AWindows)

```
 _____     ______________    __________      ___________________    ___
|     \   /              \  |          |    |                   |  |   |
|      \_/       __       \_|    __    |    |    ___     ___    |__|   |
|               |  |            |  |   |    |   |   |   |   |          |
|   |\     /|   |__|    _       |__|   |____|   |   |   |   |    __    |
|   | \___/ |          | \                      |   |   |   |   |  |   |
|___|       |__________|  \_____________________|   |___|   |___|  |___|
```

A data hoarder’s dream come true: bundle any web page into a single HTML file. You can finally replace that gazillion of open tabs with a gazillion of .html files stored somewhere on your precious little drive.

Unlike the conventional “Save page as”, `monolith` not only saves the target document, it embeds CSS, image, and JavaScript assets **all at once**, producing a single HTML5 document that is a joy to store and share.

If compared to saving websites with `wget -mpk`, this tool embeds all assets as data URLs and therefore lets browsers render the saved page exactly the way it was on the Internet, even when no network connection is available.

---------------------------------------------------

## Installation

#### Via Homebrew (on macOS and GNU/Linux)
    $ brew install monolith

#### Using Snapcraft (on GNU/Linux)
    $ snap install monolith

#### Using Ports collection (on FreeBSD and TrueOS)
    $ cd /usr/ports/www/monolith/
    $ make install clean

#### From source

Dependency: `libssl-dev`

    $ git clone https://github.com/Y2Z/monolith.git
    $ cd monolith
    $ make install

#### Using Containers
The guide can be found [here](docs/containers.md)

---------------------------------------------------

## Usage
    $ monolith https://lyrics.github.io/db/P/Portishead/Dummy/Roads/ -o portishead-roads-lyrics.html

---------------------------------------------------

## Options
 - `-c`: Ignore styles
 - `-f`: Exclude frames
 - `-F`: Omit web fonts
 - `-i`: Remove images
 - `-I`: Isolate the document
 - `-j`: Exclude JavaScript
 - `-k`: Accept invalid X.509 (TLS) certificates
 - `-o`: Write output to file
 - `-s`: Be quiet
 - `-t`: Adjust network request timeout
 - `-u`: Provide custom User-Agent

---------------------------------------------------

## HTTPS and HTTP proxies
Please set `https_proxy`, `http_proxy`, and `no_proxy` environment variables.

---------------------------------------------------

## Contributing
Please open an issue if something is wrong, that helps make this project better.

---------------------------------------------------

## Related projects
 - `Monolith Chrome Extension`: https://github.com/rhysd/monolith-of-web
 - `Pagesaver`: https://github.com/distributed-mind/pagesaver
 - `Personal WayBack Machine`: https://github.com/popey/pwbm

---------------------------------------------------

## License
The Unlicense

---------------------------------------------------

<!-- Microtext -->
<sub>Keep in mind that `monolith` is not aware of your browser’s session</sub>
