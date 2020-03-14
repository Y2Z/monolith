1. Run `docker build -t y2z/monolith .` to create a Docker image

2. Create a file named `monolith` which contains:
```sh
#!/bin/sh

docker run --rm \
    y2z/monolith \
    monolith \
    "$@"
```
3. Make the file executable (`chmod +x monolith`) and include it into your `$PATH`
4. Now you should be able to run a containerized build of monolith like this:  
`monolith -I https://github.com > document.html`

