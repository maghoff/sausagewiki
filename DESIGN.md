Sausagewiki is a simple, self-contained wiki engine. This is not merely a
description, but in fact design goals. Let's look closer at them.

Sausagewiki is simple
=====================
Sausagewiki is somewhat feature sparse. Of course, implementing features takes
time and effort, but that is not all. Sausagewiki attempts to attain
simplicity in part by being very restrictive with which features to add. Which
features belong in Sausagewiki? That is hard to pin down. There are many
features that could be included in a simple wiki engine, but it is probably
true that including _all_ those features would make the engine no longer
simple. Because of this, Sausagewiki is extremely restrictive with which
features to add.

Sausagewiki is simple to use. The user interface is clean and simple. The wiki
language is small and easy to fully grasp. For example: In order to keep the
wiki syntax small and easy, the inline HTML feature of Markdown has been
excluded. It would make the wiki syntax too large.

The executable has few command line options. It is easy for the system
administrator to get up and running correctly. As a user you can also be sure
that Sausagewiki is the same when using it in different places.

Sausagewiki is self-contained
=============================
The binary does not have runtime dependencies. It does not dynamically link
with any library. (This is true for Linux. For other systems it needs to link
to the C standard library) It does not require any other program during
run time, and it does not require any resource file external to the
Sausagewiki binary.

Sausagewiki has no configuration file. This is sensible only as long as it
also has very few command line options.

The data for a given wiki instance is contained in one file only. This makes
a wiki instance easy to back up, copy and move.

Sausagewiki is a wiki engine
============================
The user experience is geared towards collaborative editing rather than a
division between editors and readers. Sausagewiki aims for a low barrier to
entry; new readers should feel invited to edit the text if appropriate.
