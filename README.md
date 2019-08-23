# monolith

A data hoarder's dream come true: bundle any web page into a single HTML file.  
You can finally replace that gazillion of open tabs with a gazillion of .html files stored somewhere on your precious little drive.

Unlike conventional "Save page as…", `monolith` not only saves the target document,
it embeds CSS, image, and JavaScript assets **all at once**, producing a single HTML5 document that is a joy to store and share.

If compared to saving websites with `wget -mpk`, `monolith` embeds all assets as data URLs and therefore displays the saved page exactly the same, being completely separated from the Internet.

<!-- `This program works both on remote and local targets. -->

### Installation
    $ git clone https://github.com/Y2Z/monolith.git
    $ cd monolith
    $ cargo install

### Usage
    $ monolith https://lyrics.github.io/db/p/portishead/dummy/roads/ > portishead-roads-lyrics.html
<!-- or -->

<!-- cat local.html | monolith - > local.html -->

### Options
 - `-j`: Exclude JavaScript
 - `-i`: Remove images
<!--  - `-a`: Don't make anchors link to remote documents -->

### License
The Unlicense

<!-- Microtext -->
<sub>Keep in mind that `monolith` is not aware of your browser's session</sub>
