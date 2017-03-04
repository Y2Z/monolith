# monolith
A data hoarder's dream come true: bundle any web page into a stand-alone HTML file.  
You can finally replace that gazillion of open tabs with a gazillion of .html files  
stored somewhere on your precious little drive.

Unlike conventional "Save page as …", `monolith` not only saves the target document,
but embeds JavaScript, CSS and image assets **all at once**, resulting  
in a single HTML5 document that is a joy to store and share.

Works both on remote and local targets.

If compared to saving websites with `wget -mpk`, `monolith` embeds all assets  
as data-URIs and therefore would display the page exactly the same at any given time,  
being completely independent from the Internet.

However, keep in mind that `monolith` is not aware of your browser's session.

### Installing/Updating
    $ sudo npm install -g https://github.com/Y2Z/monolith.git

### Usage
    $ monolith https://github.com > github.html
<!-- or -->

<!--     cat local.html | monolith - > local.html -->

### Options
 - `-u`: output the result as one big data-URI
 - `-q`: be quiet
<!--  - `-a`: fix anchor href="" attributes for remote documents -->

### License
GPLv3
