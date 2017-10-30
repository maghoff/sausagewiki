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

 * Simple to [install and run](#install-and-run)
    * No runtime dependencies
 * Simple to backup, just copy the single database file whenever
 * Snappy, light resource usage
 * Markdown syntax
    * Plus table-support
    * Without embedded HTML
 * Full text search
 * Responsive design: fits different screens
 * Progressive enhancement: works with or without JavaScript

Install and run
===============
Sausagewiki aims to be simple to get up and running. It is distributed as a
single independent executable for Linux.

 1. Download the latest build of `sausagewiki.xz` from <https://github.com/maghoff/sausagewiki/releases/latest>
 2. Decompress: `xz -d sausagewiki.xz`
 3. Set execution permission: `chmod a+x sausagewiki`
 4. Run: `./sausagewiki wiki.db`

For other platforms you will presently have to build it yourself. Sausagewiki
is built like other Rust projects, with `cargo build`.

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
        -p, --port <port>    Sets the listening port

    ARGS:
        <DATABASE>    Sets the database file to use

Sausagewiki will create an SQLite database file with the filename given in the
`DATABASE` parameter and open an HTTP server bound to `127.0.0.1` and the given
port number. The default port number is 8080.
