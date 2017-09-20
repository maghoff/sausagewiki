CREATE TABLE article_revisions_copy (
    article_id INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    title TEXT NOT NULL,
    body TEXT NOT NULL,

    PRIMARY KEY (article_id, revision),
    FOREIGN KEY (article_id) REFERENCES articles(id)
);

INSERT INTO article_revisions_copy SELECT * FROM article_revisions;

DROP TABLE article_revisions;

CREATE TABLE article_revisions (
    sequence_number INTEGER PRIMARY KEY NOT NULL,

    article_id INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,

    -- Actually a synthetic property, namely revision = MAX(revision)
    -- GROUP BY article_id, but SQLite makes that so hard to work with:
    latest BOOLEAN NOT NULL,

    FOREIGN KEY (article_id) REFERENCES articles(id)
);

CREATE UNIQUE INDEX unique_revision_per_article_id ON article_revisions
    (article_id, revision);

CREATE UNIQUE INDEX unique_latest_revision_per_article_id ON article_revisions
    (article_id) WHERE latest=1;

CREATE INDEX slug_lookup ON article_revisions
    (slug, revision);


INSERT INTO article_revisions SELECT
    ROWID,
    article_id,
    revision,
    created,
    CAST(article_id AS TEXT) AS slug,
    title,
    body,
    0
FROM article_revisions_copy;

UPDATE article_revisions
    SET latest = 1
    WHERE (article_id, revision) IN (
        SELECT article_id, MAX(revision) AS revision
        FROM article_revisions
        GROUP BY article_id
    );

CREATE UNIQUE INDEX slugs_index ON article_revisions (slug, article_id) WHERE latest=1;
