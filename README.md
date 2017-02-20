# monolith
A data hoarder's dream come true:  
bundle any web page into a stand-alone HTML file.

Unlike conventional "Save page as …", `monolith` saves the target  
document **and** embeds JavaScript, CSS and image assets **all at once**,  
resulting in a single HTML5 document that is easy to store and share.

Works both on remote and local targets.

If compared to saving websites with `wget -mpk`,  
`monolith` embeds all assets as data-URIs and therefore would display the page  
exactly the same at any time, not depending on the Internet connection.

However, keep in mind that `monolith` is not aware of your browser's session.

### Installation
    $ sudo npm install -g git@github.com:Y2Z/monolith.git

### Usage
    $ monolith https://github.com > github.html
or

    $ monolith -q [local path]/index.html > mysite.html

<!-- or -->
<!--     cat local.html | monolith - > local.html -->

### Options
 - `-u`: output the result document as one big data-URI
 - `-q`: be quiet
<!--  - `-a`: fix anchor href="" attributes for remote documents -->

### License
GPLv3
