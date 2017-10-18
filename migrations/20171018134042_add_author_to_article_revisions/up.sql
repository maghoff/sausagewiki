ALTER TABLE article_revisions ADD COLUMN author TEXT CHECK (author != '');

CREATE INDEX author_lookup ON article_revisions (author);
