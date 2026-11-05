[![monolith build status on GNU/Linux](https://github.com/Y2Z/monolith/workflows/GNU%2FLinux/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AGNU%2FLinux)
[![monolith build status on macOS](https://github.com/Y2Z/monolith/workflows/macOS/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AmacOS)
[![monolith build status on Windows](https://github.com/Y2Z/monolith/workflows/Windows/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AWindows)
[![Monolith Actor on Apify](https://apify.com/actor-badge?actor=snshn/monolith)](https://apify.com/snshn/monolith?fpr=snshn)


```
 _____    _____________   __________     ___________________    ___
|     \  /             \ |          |   |                   |  |   |
|      \/       __      \|    __    |   |    ___     ___    |__|   |
|              |  |          |  |   |   |   |   |   |   |          |
|   |\    /|   |__|          |__|   |___|   |   |   |   |    __    |
|   | \__/ |          |\                    |   |   |   |   |  |   |
|___|      |__________| \___________________|   |___|   |___|  |___|
```

A data hoarder’s dream come true: bundle any web page into a single HTML file. You can finally replace that gazillion of open tabs with a gazillion of .html files stored somewhere on your precious little drive.

Unlike the conventional “Save page as”, `monolith` not only saves the target document, it embeds CSS, image, and JavaScript assets **all at once**, producing a single HTML5 document that is a joy to store and share.

If compared to saving websites with `wget -mpk`, this tool embeds all assets as data URLs and therefore lets browsers render the saved page exactly the way it was on the Internet, even when no network connection is available.

![Monolith GUI screenshot](assets/screenshots/gui.png)

---------------------------------------------------


## Installation

#### Using [Cargo](https://crates.io/crates/monolith) (cross-platform)

```console
cargo install monolith
```

#### Via [Homebrew](https://formulae.brew.sh/formula/monolith) (macOS and GNU/Linux)

```console
brew install monolith
```

#### Via [Chocolatey](https://community.chocolatey.org/packages/monolith) (Windows)

```console
choco install monolith
```

#### Via [Scoop](https://scoop.sh/#/apps?q=monolith) (Windows)

```console
scoop install main/monolith
```

#### Via [Winget](https://winstall.app/apps/Y2Z.Monolith) (Windows)

```console
winget install --id=Y2Z.Monolith -e
```

#### Via [MacPorts](https://ports.macports.org/port/monolith/summary) (macOS)

```console
sudo port install monolith
```

#### Using [Snapcraft](https://snapcraft.io/monolith) (GNU/Linux)

```console
snap install monolith
```

#### Using [Guix](https://packages.guix.gnu.org/packages/monolith) (GNU/Linux)

```console
guix install monolith
```

#### Using [NixPkgs](https://search.nixos.org/packages?channel=unstable&show=monolith&query=monolith)

```console
nix-env -iA nixpkgs.monolith
```

#### Using [Flox](https://flox.dev)

```console
flox install monolith
```

#### Using [X-CMD](https://www.x-cmd.com/pkg/monolith)

```console
x install monolith
```

#### Using [Pacman](https://archlinux.org/packages/extra/x86_64/monolith) (Arch Linux)

```console
pacman -S monolith
```

#### Using [aports](https://pkgs.alpinelinux.org/packages?name=monolith) (Alpine Linux)

```console
apk add monolith
```

#### Using [pkg.haus APT archive](https://pkg.haus) (Debian)

Set up the archive per the instructions on [pkg.haus](https://pkg.haus), then:

```console
sudo apt install monolith
```

#### Using [XBPS Package Manager](https://voidlinux.org/packages/?q=monolith) (Void Linux)

```console
xbps-install -S monolith
```

#### Using [FreeBSD packages](https://svnweb.freebsd.org/ports/head/www/monolith/) (FreeBSD)

```console
pkg install monolith
```

#### Using [FreeBSD ports](https://www.freshports.org/www/monolith/) (FreeBSD)

```console
cd /usr/ports/www/monolith/
make install clean
```

#### Using [pkgsrc](https://pkgsrc.se/www/monolith) (NetBSD, OpenBSD, Haiku, etc)

```console
cd /usr/pkgsrc/www/monolith
make install clean
```

#### Using [containers](https://www.docker.com/)

```console
docker build -t y2z/monolith .
sudo install -b dist/run-in-container.sh /usr/local/bin/monolith
```

#### From [source](https://github.com/Y2Z/monolith)

Dependencies: `cargo`

<details>
  <summary>Install cargo (GNU/Linux)</summary>
 Check if cargo is installed

 ```console
 cargo -v
 ```

 If cargo is not already installed, install and add it to your existing ```$PATH``` (paraphrasing the [official installation instructions](https://doc.rust-lang.org/cargo/getting-started/installation.html)):

 ```console
 curl https://sh.rustup.rs -sSf | sh
 . "$HOME/.cargo/env"
 ```

Proceed with installing from source:
</details>

```console
git clone https://github.com/Y2Z/monolith.git
cd monolith
make install
```

#### Using [pre-built binaries](https://github.com/Y2Z/monolith/releases) (Windows, ARM-based devices, etc)

Every release contains pre-built binaries for Windows, GNU/Linux, as well as platforms with non-standard CPU architecture.


---------------------------------------------------


## Usage

```console
monolith https://lyrics.github.io/db/P/Portishead/Dummy/Roads/ -o %title%.%timestamp%.html
```

```console
cat some-site-page.html | monolith -IifsM -r audio,video -w frame,iframe -b https://some.site/ - > some-site-page-with-assets.html
```

```console
monolith page-saved-by-browser.mhtml -o page.html
```


---------------------------------------------------


## Options

 - `-b`: Use `custom base URL`
 - `-B`: Let a browser render the page first (requires Firefox)
 - `-C`: Read cookies from `file`
 - `-d`: Forbid retrieving assets from specified `domain(s)` (can be used multiple times)
 - `-D`: Create missing directories in the output file path
 - `-e`: Ignore network errors
 - `-E`: Save document using `custom encoding`
 - `-f`: Exclude web fonts
 - `-h`: Print usage information
 - `-i`: Remove images
 - `-I`: Isolate the document
 - `-j`: Exclude JavaScript
 - `-k`: Accept invalid X.509 (TLS) certificates
 - `-m`: Output in MHTML format instead of HTML
 - `-M`: Don't add timestamp and URL information
 - `-n`: Extract contents of NOSCRIPT elements
 - `-o`: Write output to `file` (use “-” for STDOUT)
 - `-q`: Be quiet
 - `-r`: Remove elements matching `selector` from the document (can be used multiple times)
 - `-R`: Save a simplified version of the page, like the reader mode of a web browser: just the article, without any JavaScript, and with styling that follows the system's color scheme (add `-s` to leave it without any styles)
 - `-s`: Exclude styles
 - `-t`: Adjust `network request timeout`
 - `-u`: Use `custom User-Agent`
 - `-w`: Wipe elements matching `selector`: links and such turn into their contents, images, frames, and other elements that load something lose their sources (can be used multiple times)
 - `-W`: Only retrieve assets from domains specified with `-d`
 - `-V`: Print version number


---------------------------------------------------


## Blacklisting and whitelisting domains

Option `-d` forbids retrieving assets from the specified domains, and adding `-W` flips it to allow only those domains, e.g.:

```console
monolith -I -d .googleusercontent.com -d googleanalytics.com -d .google.com https://example.com -o example-no-ads.html
```

```console
monolith -I -W -d example.com -d www.example.com https://example.com -o example-only.html
```


---------------------------------------------------


## Dynamic content

Monolith doesn't feature a JavaScript engine, but it can make use of the one inside of a browser that's already installed: option `-B` opens the page in Firefox, waits for it to finish rendering, and saves what the page has become, e.g.:

```console
monolith -B https://example.com -o example.html
```

The browser runs without a window, from a clean temporary profile which gets deleted afterwards. It never gets access to your own Firefox profile, cookies, or history, gets only those cookies that were passed in using `-C`, and sticks to the same rules as the rest of Monolith: domains forbidden with `-d` and whatever got excluded with `-i`, `-f`, or `-s` don't get requested by the browser either. It identifies itself as what it is, unless told otherwise with `-u`. Set `MONOLITH_BROWSER` to the browser's path to use a specific installation, or a browser based on Firefox, such as LibreWolf or GNU IceCat. Chromium and browsers based on it aren't supported yet.

The browser follows your system's light or dark mode, so pages that adapt to it get saved the way you'd see them. Set `MONOLITH_BROWSER_COLOR_SCHEME` to `dark`, `light`, or `auto` to choose for yourself.

If Monolith gets interrupted, it takes the browser and its temporary files down with it.

Should the page turn out to be a challenge which doesn't solve itself, such as the ones used to tell people from bots, the browser reopens in a window and waits for you to solve it.

The saved page keeps its JavaScript, along with the responses it has received, which get replayed to it without any network access: buttons, menus, and other things that only rely on the page's own code keep working, but anything that needs a server doesn't. A saved page only contains what it has loaded while being saved, so parts of web apps that were never opened won't be there. Scripts of the saved page get modified to make that work: they're told the page's original URL, and imports get pointed at what's stored inside of the document. They never reach out to the server, and `-b` doesn't apply to them. Add `-j` to save a frozen copy of what the browser had on screen instead; that's also what MHTML output and `-R` always do.

The same is available to the GUI (“Render in browser”) and to programs using Monolith as a library, by setting `browser` in `MonolithOptions`. Apps that have a web view of their own, such as mobile ones, can use `Session::add_asset()` and `create_monolithic_document_from_data()` to save what it has rendered.

Any other browser can be used as a pre-processor as well, e.g. Chromium (Chrome):

```console
chromium --headless --window-size=1920,1080 --run-all-compositor-stages-before-draw --virtual-time-budget=9000 --incognito --dump-dom https://github.com | monolith - -I -b https://github.com -o github.html
```


---------------------------------------------------


## Authentication

```console
monolith https://username:password@example.com -o example-basic-auth.html
```


---------------------------------------------------


## Proxies

Please set `https_proxy`, `http_proxy`, and `no_proxy` environment variables.


---------------------------------------------------

### Apify Actor Usage

<a href="https://apify.com/snshn/monolith?fpr=snshn"><img src="https://apify.com/ext/run-on-apify.png" alt="Run Monolith Actor on Apify" width="176" height="39" /></a>

You can run Monolith in the cloud without installation using the [Monolith Actor](https://apify.com/snshn/monolith?fpr=snshn) on [Apify](https://apify.com?fpr=snshn) free of charge.

``` bash
echo '{"urls": ["https://news.ycombinator.com/"]}' | apify call -so snshn/monolith
[{
  "url": "https://news.ycombinator.com/",
  "status": "0",
  "kvsUrl": "https://api.apify.com/v2/key-value-stores/of9xNgvpon4elPLbc/records/https___news.ycombinator.com_"
}]
```

Read more about the [Monolith Actor](.actor/README.md), including how to use it via the Apify UI, API and CLI without installation.

---------------------------------------------------


## Contributing

Please open an issue if something is wrong, that helps make this project better.


---------------------------------------------------


## License

To the extent possible under law, the author(s) have dedicated all copyright related and neighboring rights to this software to the public domain worldwide.
This software is distributed without any warranty.
