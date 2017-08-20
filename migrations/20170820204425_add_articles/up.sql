CREATE TABLE articles (
    id INTEGER PRIMARY KEY NOT NULL
);

CREATE TABLE article_revisions (
    article_id INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    title TEXT NOT NULL,
    body TEXT NOT NULL,

    PRIMARY KEY (article_id, revision),
    FOREIGN KEY (article_id) REFERENCES articles(id)
);

