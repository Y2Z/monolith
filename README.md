[![monolith build status on GNU/Linux](https://github.com/Y2Z/monolith/workflows/GNU%2FLinux/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AGNU%2FLinux)
[![monolith build status on macOS](https://github.com/Y2Z/monolith/workflows/macOS/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AmacOS)
[![monolith build status on Windows](https://github.com/Y2Z/monolith/workflows/Windows/badge.svg)](https://github.com/Y2Z/monolith/actions?query=workflow%3AWindows)

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

#### Using [Pacman](https://archlinux.org/packages/extra/x86_64/monolith) (Arch Linux)

```console
pacman -S monolith
```

#### Using [aports](https://pkgs.alpinelinux.org/packages?name=monolith) (Alpine Linux)

```console
apk add monolith
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

Dependencies: `libssl`, `cargo`

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
monolith https://lyrics.github.io/db/P/Portishead/Dummy/Roads/ -o portishead-roads-lyrics.html
```

```console
cat some-site-page.html | monolith -aIiFfcMv -b https://some.site/ - > some-site-page-with-assets.html
```


---------------------------------------------------


## Options

 - `-a`: Exclude audio sources
 - `-b`: Use `custom base URL`
 - `-B`: Forbid retrieving assets from specified domain(s)
 - `-c`: Exclude CSS
 - `-C`: Read cookies from `file`
 - `-d`: Allow retrieving assets only from specified `domain(s)`
 - `-e`: Ignore network errors
 - `-E`: Save document using `custom encoding`
 - `-f`: Omit frames
 - `-F`: Exclude web fonts
 - `-h`: Print help information
 - `-i`: Remove images
 - `-I`: Isolate the document
 - `-j`: Exclude JavaScript
 - `-k`: Accept invalid X.509 (TLS) certificates
 - `-M`: Don't add timestamp and URL information
 - `-n`: Extract contents of NOSCRIPT elements
 - `-o`: Write output to `file` (use “-” for STDOUT)
 - `-s`: Be quiet
 - `-t`: Adjust `network request timeout`
 - `-u`: Provide `custom User-Agent`
 - `-v`: Exclude videos
 - `-V`: Print version number


---------------------------------------------------


## Whitelisting and blacklisting domains

Options `-d` and `-B` provide control over what domains can be used to retrieve assets from, e.g.:

```console
monolith -I -d example.com -d www.example.com https://example.com -o example-only.html
```

```console
monolith -I -B -d .googleusercontent.com -d googleanalytics.com -d .google.com https://example.com -o example-no-ads.html
```

---------------------------------------------------


## Dynamic content

Monolith doesn't feature a JavaScript engine, hence websites that retrieve and display data after initial load may require usage of additional tools.

For example, Chromium (Chrome) can be used to act as a pre-processor for such pages:

```console
chromium --headless --window-size=1920,1080 --run-all-compositor-stages-before-draw --virtual-time-budget=9000 --incognito --dump-dom https://github.com | monolith - -I -b https://github.com -o github.html
```


---------------------------------------------------


## Proxies

Please set `https_proxy`, `http_proxy`, and `no_proxy` environment variables.


---------------------------------------------------


## Contributing

Please open an issue if something is wrong, that helps make this project better.


---------------------------------------------------


## License

To the extent possible under law, the author(s) have dedicated all copyright related and neighboring rights to this software to the public domain worldwide.
This software is distributed without any warranty.
