DROP INDEX slugs_index;
CREATE UNIQUE INDEX slugs_index ON article_revisions (slug) WHERE latest=1;
