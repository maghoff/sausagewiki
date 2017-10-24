CREATE VIRTUAL TABLE article_search USING fts5(
    title,
    body,
    slug UNINDEXED,
    tokenize = 'porter unicode61 remove_diacritics 1'
);

INSERT INTO article_search(title, body, slug)
    SELECT title, body, slug FROM article_revisions WHERE latest != 0;

CREATE TRIGGER article_revisions_ai AFTER INSERT ON article_revisions WHEN new.latest != 0 BEGIN
  DELETE FROM article_search WHERE rowid = new.article_id;
  INSERT INTO article_search(rowid, title, body, slug) VALUES (new.article_id, new.title, new.body, new.slug);
END;
CREATE TRIGGER article_revisions_ad AFTER DELETE ON article_revisions WHEN old.latest != 0 BEGIN
  DELETE FROM article_search WHERE rowid = old.article_id;
END;

-- Index unique_latest_revision_per_article_id makes sure the following is sufficient:
CREATE TRIGGER article_revisions_au_disable AFTER UPDATE ON article_revisions WHEN old.latest != 0 AND new.latest == 0 BEGIN
  DELETE FROM article_search WHERE rowid = old.article_id;
END;
CREATE TRIGGER article_revisions_au_enable AFTER UPDATE ON article_revisions WHEN old.latest == 0 AND new.latest != 0 BEGIN
  INSERT INTO article_search(rowid, title, body, slug) VALUES (new.article_id, new.title, new.body, new.slug);
END;
