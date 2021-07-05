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

#### Using [Cargo](https://crates.io/crates/monolith)
    $ cargo install monolith

#### Via [Homebrew](https://formulae.brew.sh/formula/monolith) (macOS and GNU/Linux)
    $ brew install monolith

#### Via [MacPorts](https://ports.macports.org/port/monolith/summary) (macOS)
    $ sudo port install monolith

#### Using [Snapcraft](https://snapcraft.io/monolith) (GNU/Linux)
    $ snap install monolith

#### Using [FreeBSD packages](https://svnweb.freebsd.org/ports/head/www/monolith/) (FreeBSD)
    $ pkg install monolith

#### Using [FreeBSD ports](https://www.freshports.org/www/monolith/) (FreeBSD)
    $ cd /usr/ports/www/monolith/
    $ make install clean

#### Using [pkgsrc](https://pkgsrc.se/www/monolith) (NetBSD, OpenBSD, Haiku, etc)
    $ cd /usr/pkgsrc/www/monolith
    $ make install clean

#### Using [pre-built binaries](https://github.com/Y2Z/monolith/releases) (Windows, ARM-based devices, etc)
Every release contains pre-built binaries for Windows, GNU/Linux, as well as platforms with non-standart CPU architecture.

#### Using [containers](https://www.docker.com/)
    $ docker build -t Y2Z/monolith .
    $ sudo install -b utils/run-in-container.sh /usr/local/bin/monolith

#### From source

Dependency: `libssl-dev`

    $ git clone https://github.com/Y2Z/monolith.git
    $ cd monolith
    $ make install

---------------------------------------------------

## Usage
    $ monolith https://lyrics.github.io/db/P/Portishead/Dummy/Roads/ -o portishead-roads-lyrics.html
or

    $ cat index.html | monolith -aIiFfcMv -b https://original.site/ - > result.html

---------------------------------------------------

## Options
 - `-a`: Exclude audio sources
 - `-b`: Use custom `base URL`
 - `-c`: Exclude CSS
 - `-C`: Save document using custom `charset`
 - `-e`: Ignore network errors
 - `-f`: Omit frames
 - `-F`: Exclude web fonts
 - `-i`: Remove images
 - `-I`: Isolate the document
 - `-j`: Exclude JavaScript
 - `-k`: Accept invalid X.509 (TLS) certificates
 - `-M`: Don't add timestamp and URL information
 - `-n`: Extract contents of NOSCRIPT elements
 - `-o`: Write output to `file`, use “-” for STDOUT
 - `-s`: Be quiet
 - `-t`: Adjust `network request timeout`
 - `-u`: Provide custom `User-Agent`
 - `-v`: Exclude videos

---------------------------------------------------

## Proxies
Please set `https_proxy`, `http_proxy`, and `no_proxy` environment variables.

---------------------------------------------------

## Contributing
Please open an issue if something is wrong, that helps make this project better.

---------------------------------------------------

## Related projects
 - Monolith Chrome Extension: https://github.com/rhysd/monolith-of-web
 - Pagesaver: https://github.com/distributed-mind/pagesaver
 - Personal WayBack Machine: https://github.com/popey/pwbm
 - Hako: https://github.com/dmpop/hako
 - Monk: https://gitlab.com/fisherdarling/monk

---------------------------------------------------

## License

To the extent possible under law, the author(s) have dedicated all copyright related and neighboring rights to this software to the public domain worldwide.
This software is distributed without any warranty.

---------------------------------------------------

<!-- Microtext -->
<sub>Keep in mind that `monolith` is not aware of your browser’s session</sub>
