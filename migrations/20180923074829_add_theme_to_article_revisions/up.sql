ALTER TABLE article_revisions ADD COLUMN theme TEXT NOT NULL CHECK (theme IN (
    'red', 'pink', 'purple', 'deep-purple', 'indigo', 'blue', 'light-blue',
    'cyan', 'teal', 'green', 'light-green', 'lime', 'yellow', 'amber',
    'orange', 'deep-orange', 'brown', 'gray', 'blue-gray'
)) DEFAULT 'red';

UPDATE article_revisions SET theme=theme_from_str_hash(title);
