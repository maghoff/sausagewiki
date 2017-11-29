[![Build Status](https://travis-ci.org/maghoff/sausagewiki.svg?branch=master)](https://travis-ci.org/maghoff/sausagewiki)

Sausagewiki is a simple, self-contained wiki engine.

Copyright (C) 2017 Magnus Hovland Hoff <maghoff@gmail.com>

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version.

Features
========
Some features of Sausagewiki, in no particular order:

 * Simple to [install and run](#install-and-run), no runtime dependencies
 * Reasonable to [back up](https://github.com/maghoff/sausagewiki/wiki/Backup),
   there is only one data file
 * Can be reverse proxied behind any path without configuration
 * Snappy, light resource usage
 * Markdown syntax
    * Plus [table-support](https://help.github.com/articles/organizing-information-with-tables/)
    * Without embedded HTML
 * Full text search
 * Responsive design: fits different screens as well as print
 * Progressive enhancement: works with or without JavaScript
 * Works with [external authentication](https://github.com/maghoff/sausagewiki/wiki/Enabling-authentication)

Install and run
===============
Sausagewiki aims to be simple to get up and running. It is distributed as a
single independent executable for Linux.

 1. Download the latest build of `sausagewiki.xz` from <https://github.com/maghoff/sausagewiki/releases/latest>
 2. Decompress: `xz -d sausagewiki.xz`
 3. Set execution permission: `chmod a+x sausagewiki`
 4. Run: `./sausagewiki wiki.db`

For a more permanent installation, you could [install Sausagewiki as a systemd
service](https://github.com/maghoff/sausagewiki/wiki/System-wide-installation-with-systemd).

Alternatively, you could build it from source code with `cargo build`.

Command line arguments
----------------------
    USAGE:
        sausagewiki [FLAGS] [OPTIONS] <DATABASE>

    FLAGS:
        -h, --help              Prints help information
            --trust-identity    Trust the value in the X-Identity header to be an authenticated username.
                                This only makes sense when Sausagewiki runs behind a reverse proxy which
                                sets this header.
        -V, --version           Prints version information

    OPTIONS:
        -a, --address <address>    Sets the IP address to bind to [default: 127.0.0.1]
        -p, --port <port>          Sets the listening port [default: 8080]

    ARGS:
        <DATABASE>    Sets the database file to use

Sausagewiki will create an SQLite database file with the filename given in the
`DATABASE` parameter and open an HTTP server bound to the configured address,
`<address>:<port>`.
