# gitomato
Small and simple HTTP server for static webpages stored in git repositories.  
The pages will be automatic updated when the git repo changes.  
It does **not** support scripting or automatic transformations.

## Features
[x] Serve one or more pages from different git repositories.  
[x] You can select a specific branch or use the default one.  
[x] Uses a shallow clone to save disk space and bandwidth.  
[x] All files are kept in memory to avoid disk IO when service pages.  
[x] Can serve only selected subfolder os the repository.  
[x] Update pages via regular update intervals or use web hooks.  
[x] Can also generate directory index pages (default is off).  
[x] Supports Client-side caching using ETag based on file hash.  
[x] Compiled to a single statically linked executable.  
[x] Extremely small Docker container (less than 10 MB).  

## Configuration
See `gitomato --help` for all global parameters.  
Run `gitomato` without any arguments for an page setup tutorial.

The simplest possible example to configure gitomato is this:
`gitomato --page-git-repo=https://git.server.org/my-repo.git`

### Docker
You can also use the provided Docker image to deploy gitomato.
When using Docker, the recommended way of configuration are environment variables.
Here a more complex example with two pages:
```
docker run --rm -p 8080:8080 \
    -e PAGE0_GIT_REPO="https://user:pass@server.org/my-repo.git" \
    -e PAGE0_GIT_REF="master" \
    -e PAGE0_PREFIX="/page0/" \
    -e PAGE0_GIT_SUBFOLDER="htdocs/" \
    -e PAGE0_AUTO_LIST=true \
    -e PAGE0_UPDATE_SECRET="secr3t" \
    -e PAGE0_MAX_BYTES=10000000 \
    -e PAGE1_GIT_REPO="https://server.com/another-repo.git" \
    -e PAGE1_PREFIX="/page1/" \
    ghcr.io/cry-inc/gitomato
```

### Automatic Updates
There are two ways to update your pages when the git repository changes.
The default option are automatic update intervals.
The server will check the repos in configured intervals for any updates.
If you run the default configuration, the update check interval is 300 seconds.
You can change this using the global configuration option `--interval` or the environment variable `INTERVAL`.

The second option is an web hook that can be called by your [git forge](https://en.wikipedia.org/wiki/Forge_(software)) whenever new data is pushed.
This will result in immediate updates, but requires extra setup.
You have to enable the update secret for your page, for example using `--page-update-secret=123`.
This enables a HTTP GET endpoint below your page root at `/update/123`.
You can then use this secret URL to set up the web hook in your git forge.

# Why?
Why not just use GitHub Pages?

I wanted a solution that works for any git repository accessible over HTTP(S).
And I wanted to be able to selfhost all parts.
Additionally, other existing solutions like [pages-server](https://codeberg.org/Codeberg/pages-server) or [git-pages](https://codeberg.org/git-pages/git-pages) are too big and complex for my use case.
