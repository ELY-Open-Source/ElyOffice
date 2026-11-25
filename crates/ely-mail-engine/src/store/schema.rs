/// Step `n` upgrades version `n`.
pub(super) const STEPS: &[&str] = &[V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, V12, V13, V14, V15, V16, V17, V18, V19];

/// Upgrades to this version reindex every message.
pub(super) const REINDEX: usize = 12;
/// Stable identities are filled from originals at this version.
pub(super) const IDENTS: usize = 18;

const V1: &str = "
CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    address TEXT NOT NULL UNIQUE COLLATE NOCASE,
    settings TEXT NOT NULL
);
CREATE TABLE mailboxes (
    id INTEGER PRIMARY KEY,
    account INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE,
    name TEXT NOT NULL,
    UNIQUE (account, name)
);
CREATE TABLE messages (
    id INTEGER PRIMARY KEY,
    mailbox INTEGER NOT NULL REFERENCES mailboxes ON DELETE CASCADE,
    uidvalidity INTEGER,
    uid INTEGER,
    uidl TEXT,
    message_id TEXT,
    in_reply_to TEXT NOT NULL,
    refs TEXT NOT NULL,
    subject TEXT,
    sender TEXT NOT NULL,
    recipients TEXT NOT NULL,
    copies TEXT NOT NULL,
    date INTEGER,
    attachments INTEGER,
    content TEXT,
    CHECK ((uid IS NULL) = (uidvalidity IS NULL) AND (uid IS NULL) <> (uidl IS NULL))
);
CREATE UNIQUE INDEX messages_imap ON messages (mailbox, uidvalidity, uid)
    WHERE uid IS NOT NULL;
CREATE UNIQUE INDEX messages_pop ON messages (mailbox, uidl) WHERE uidl IS NOT NULL;
CREATE INDEX messages_thread ON messages (message_id);
CREATE INDEX messages_content ON messages (content);
CREATE TABLE flags (
    message INTEGER NOT NULL REFERENCES messages ON DELETE CASCADE,
    flag TEXT NOT NULL,
    PRIMARY KEY (message, flag)
) WITHOUT ROWID;
CREATE INDEX flags_by_flag ON flags (flag, message);
CREATE VIRTUAL TABLE search USING fts5(
    subject, people, body, files,
    content = '', contentless_delete = 1, tokenize = 'trigram'
);
CREATE TRIGGER messages_gone AFTER DELETE ON messages BEGIN
    DELETE FROM search WHERE rowid = old.id;
END;
";

/// IMAP sync (C12.6).
const V2: &str = "
ALTER TABLE accounts ADD COLUMN synced INTEGER;
ALTER TABLE mailboxes ADD COLUMN role TEXT;
ALTER TABLE mailboxes ADD COLUMN uidvalidity INTEGER;
ALTER TABLE messages ADD COLUMN missing TEXT NOT NULL DEFAULT '[]';
";

/// Folders kept whole (C12.8).
const V3: &str = "
ALTER TABLE mailboxes ADD COLUMN offline INTEGER NOT NULL DEFAULT 0;
";

/// Ids never reused (C12.9); foreign keys are off here.
const V4: &str = "
CREATE TABLE accounts_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL UNIQUE COLLATE NOCASE,
    settings TEXT NOT NULL,
    synced INTEGER
);
CREATE TABLE mailboxes_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE,
    name TEXT NOT NULL,
    role TEXT,
    uidvalidity INTEGER,
    offline INTEGER NOT NULL DEFAULT 0,
    UNIQUE (account, name)
);
CREATE TABLE messages_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mailbox INTEGER NOT NULL REFERENCES mailboxes ON DELETE CASCADE,
    uidvalidity INTEGER,
    uid INTEGER,
    uidl TEXT,
    message_id TEXT,
    in_reply_to TEXT NOT NULL,
    refs TEXT NOT NULL,
    subject TEXT,
    sender TEXT NOT NULL,
    recipients TEXT NOT NULL,
    copies TEXT NOT NULL,
    date INTEGER,
    attachments INTEGER,
    content TEXT,
    missing TEXT NOT NULL DEFAULT '[]',
    CHECK ((uid IS NULL) = (uidvalidity IS NULL) AND (uid IS NULL) <> (uidl IS NULL))
);
INSERT INTO accounts_new (id, address, settings, synced)
    SELECT id, address, settings, synced FROM accounts;
INSERT INTO mailboxes_new (id, account, name, role, uidvalidity, offline)
    SELECT id, account, name, role, uidvalidity, offline FROM mailboxes;
INSERT INTO messages_new (id, mailbox, uidvalidity, uid, uidl, message_id, in_reply_to,
        refs, subject, sender, recipients, copies, date, attachments, content, missing)
    SELECT id, mailbox, uidvalidity, uid, uidl, message_id, in_reply_to,
        refs, subject, sender, recipients, copies, date, attachments, content, missing
    FROM messages;
DROP TABLE messages;
DROP TABLE mailboxes;
DROP TABLE accounts;
ALTER TABLE accounts_new RENAME TO accounts;
ALTER TABLE mailboxes_new RENAME TO mailboxes;
ALTER TABLE messages_new RENAME TO messages;
CREATE UNIQUE INDEX messages_imap ON messages (mailbox, uidvalidity, uid)
    WHERE uid IS NOT NULL;
CREATE UNIQUE INDEX messages_pop ON messages (mailbox, uidl) WHERE uidl IS NOT NULL;
CREATE INDEX messages_thread ON messages (message_id);
CREATE INDEX messages_content ON messages (content);
CREATE TRIGGER messages_gone AFTER DELETE ON messages BEGIN
    DELETE FROM search WHERE rowid = old.id;
END;
";

/// Senders whose remote pictures load (C12.10).
const V5: &str = "
CREATE TABLE trusted (address TEXT PRIMARY KEY COLLATE NOCASE) WITHOUT ROWID;
";

/// Headers-only messages keep their header block (C12.10).
const V6: &str = "
ALTER TABLE messages ADD COLUMN head BLOB;
-- Sync fetches these again, with their headers.
DELETE FROM flags WHERE message IN (SELECT id FROM messages WHERE content IS NULL);
DELETE FROM messages WHERE content IS NULL;
";

/// Each message's own and named Message-IDs (C12.11).
const V7: &str = "
CREATE TABLE links (
    message INTEGER NOT NULL REFERENCES messages ON DELETE CASCADE,
    id TEXT NOT NULL,
    PRIMARY KEY (id, message)
) WITHOUT ROWID;
CREATE INDEX links_by_message ON links (message);
INSERT OR IGNORE INTO links (message, id)
    SELECT id, message_id FROM messages WHERE message_id <> '';
INSERT OR IGNORE INTO links (message, id)
    SELECT m.id, j.value FROM messages m, json_each(m.in_reply_to) j WHERE j.value <> '';
INSERT OR IGNORE INTO links (message, id)
    SELECT m.id, j.value FROM messages m, json_each(m.refs) j WHERE j.value <> '';
";

/// Each account's sender name and signature (C12.12).
const V8: &str = "
CREATE TABLE identities (
    account INTEGER PRIMARY KEY REFERENCES accounts ON DELETE CASCADE,
    name TEXT,
    signature TEXT NOT NULL
);
";

/// Messages being written, and their server copies (C12.13).
const V9: &str = "
ALTER TABLE accounts ADD COLUMN drafts_up INTEGER NOT NULL DEFAULT 0;
CREATE TABLE drafts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE,
    saved INTEGER NOT NULL,
    body TEXT NOT NULL,
    remote_mailbox TEXT,
    remote_validity INTEGER,
    remote_uid INTEGER,
    CHECK ((remote_mailbox IS NULL) = (remote_uid IS NULL)
        AND (remote_uid IS NULL) = (remote_validity IS NULL))
);
CREATE TABLE draft_files (
    draft INTEGER NOT NULL REFERENCES drafts ON DELETE CASCADE,
    place INTEGER NOT NULL,
    name TEXT,
    content_type TEXT NOT NULL,
    params TEXT NOT NULL,
    content_id TEXT,
    bytes BLOB NOT NULL,
    PRIMARY KEY (draft, place)
) WITHOUT ROWID;
";

/// Messages handed to the queue, until done (C12.14).
const V10: &str = "
ALTER TABLE accounts ADD COLUMN sent_copy INTEGER;
ALTER TABLE accounts ADD COLUMN smtp_limit INTEGER;
CREATE TABLE outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN
        ('queued', 'sending', 'accepted', 'failed', 'unsure', 'cancelled')),
    filing TEXT CHECK (filing IN ('waiting', 'failed')),
    message_id TEXT NOT NULL,
    body TEXT NOT NULL,
    envelope TEXT NOT NULL,
    bytes BLOB NOT NULL,
    queued INTEGER NOT NULL,
    tried INTEGER,
    error TEXT,
    CHECK ((filing IS NULL) <> (state = 'accepted'))
);
CREATE TABLE outbox_files (
    item INTEGER NOT NULL REFERENCES outbox ON DELETE CASCADE,
    place INTEGER NOT NULL,
    name TEXT,
    content_type TEXT NOT NULL,
    params TEXT NOT NULL,
    content_id TEXT,
    bytes BLOB NOT NULL,
    PRIMARY KEY (item, place)
) WITHOUT ROWID;
";

/// Changes waiting for the server, and moves shown early (C12.15).
const V11: &str = "
ALTER TABLE messages ADD COLUMN moving INTEGER REFERENCES mailboxes ON DELETE SET NULL;
ALTER TABLE messages ADD COLUMN erasing INTEGER NOT NULL DEFAULT 0;
CREATE TABLE changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mailbox INTEGER NOT NULL REFERENCES mailboxes ON DELETE CASCADE,
    uidvalidity INTEGER NOT NULL,
    uid INTEGER NOT NULL,
    version INTEGER NOT NULL,
    seen INTEGER,
    flagged INTEGER,
    seen_was INTEGER,
    flagged_was INTEGER,
    target INTEGER REFERENCES mailboxes,
    expunge INTEGER NOT NULL DEFAULT 0,
    UNIQUE (mailbox, uidvalidity, uid),
    CHECK (NOT (expunge AND target IS NOT NULL))
);
CREATE TABLE pop_gone (
    mailbox INTEGER NOT NULL REFERENCES mailboxes ON DELETE CASCADE,
    uidl TEXT NOT NULL,
    PRIMARY KEY (mailbox, uidl)
) WITHOUT ROWID;
CREATE TABLE dropped (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('gone', 'renumbered', 'refused', 'unsupported', 'unplaced',
        'vanished')),
    words TEXT
);
";

/// Searching by word, CJK by character, sender apart (C12.16).
const V12: &str = "
DROP TABLE search;
CREATE VIRTUAL TABLE search USING fts5(
    subject, sender, recipients, body, files,
    content = '', contentless_delete = 1, tokenize = 'unicode61 remove_diacritics 2'
);
";

/// Folders pinned to the top, accounts folded (R11.3).
const V13: &str = "
ALTER TABLE mailboxes ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE accounts ADD COLUMN folded INTEGER NOT NULL DEFAULT 0;
";

/// Local rules; `handled` marks mail already ruled (C12.18).
const V14: &str = "
ALTER TABLE accounts ADD COLUMN handled INTEGER NOT NULL DEFAULT 0;
UPDATE accounts SET handled = (SELECT coalesce(max(m.id), 0) FROM messages m
    JOIN mailboxes b ON b.id = m.mailbox WHERE b.account = accounts.id);
CREATE TABLE rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE,
    field TEXT NOT NULL CHECK (field IN ('from', 'to', 'subject')),
    words TEXT NOT NULL CHECK (words <> ''),
    action TEXT NOT NULL CHECK (action IN ('move', 'flag', 'archive')),
    target INTEGER REFERENCES mailboxes ON DELETE CASCADE,
    CHECK ((action = 'move') = (target IS NOT NULL))
);
";

/// Contacts; those written to from Sent folders seed them (C12.19).
const V15: &str = "
CREATE TABLE contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (address <> ''),
    name TEXT CHECK (name <> ''),
    sent INTEGER NOT NULL DEFAULT 0,
    last INTEGER
);
INSERT INTO contacts (address, name, sent, last)
SELECT address, max(name), count(*), max(date) FROM (
    SELECT trim(json_extract(p.value, '$.email')) AS address,
           nullif(trim(json_extract(p.value, '$.name')), '') AS name, m.date
    FROM messages m JOIN mailboxes b ON b.id = m.mailbox, json_each(m.recipients) p
    WHERE b.role = 'sent'
    UNION ALL
    SELECT trim(json_extract(p.value, '$.email')), nullif(trim(json_extract(p.value, '$.name')), ''), m.date
    FROM messages m JOIN mailboxes b ON b.id = m.mailbox, json_each(m.copies) p
    WHERE b.role = 'sent'
)
WHERE address <> ''
GROUP BY address COLLATE NOCASE;
";

/// Folders imported here, never synced (C12.20); ids never reused.
const V16: &str = "
CREATE TABLE mailboxes_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account INTEGER NOT NULL REFERENCES accounts ON DELETE CASCADE,
    name TEXT NOT NULL,
    role TEXT,
    uidvalidity INTEGER,
    offline INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    local INTEGER NOT NULL DEFAULT 0,
    UNIQUE (account, local, name)
);
INSERT INTO mailboxes_new (id, account, name, role, uidvalidity, offline, pinned)
    SELECT id, account, name, role, uidvalidity, offline, pinned FROM mailboxes;
INSERT INTO sqlite_sequence (name, seq)
    SELECT 'mailboxes_new', 0 WHERE NOT EXISTS (SELECT 1 FROM sqlite_sequence WHERE name = 'mailboxes_new');
UPDATE sqlite_sequence
    SET seq = max(seq, coalesce((SELECT seq FROM sqlite_sequence WHERE name = 'mailboxes'), 0))
    WHERE name = 'mailboxes_new';
DROP TABLE mailboxes;
ALTER TABLE mailboxes_new RENAME TO mailboxes;
";

/// Rebuilt mail is no news; trimmed mail stays trimmed.
const V17: &str = "
ALTER TABLE messages ADD COLUMN trimmed INTEGER NOT NULL DEFAULT 0;
CREATE TABLE rebuilt (
    mailbox INTEGER NOT NULL REFERENCES mailboxes ON DELETE CASCADE,
    ident TEXT NOT NULL,
    PRIMARY KEY (mailbox, ident)
) WITHOUT ROWID;
";

/// Identity outlives the cache; bodies compare only when whole.
const V18: &str = "
ALTER TABLE messages ADD COLUMN ident TEXT;
CREATE TABLE rebuilt_new (
    mailbox INTEGER NOT NULL REFERENCES mailboxes ON DELETE CASCADE,
    ident TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (mailbox, ident, body)
) WITHOUT ROWID;
INSERT INTO rebuilt_new (mailbox, ident) SELECT mailbox, ident FROM rebuilt;
DROP TABLE rebuilt;
ALTER TABLE rebuilt_new RENAME TO rebuilt;
";

/// One address on two servers (#96); vault names stay.
const V19: &str = "
CREATE TABLE accounts_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL COLLATE NOCASE,
    server TEXT NOT NULL COLLATE NOCASE,
    settings TEXT NOT NULL,
    synced INTEGER,
    drafts_up INTEGER NOT NULL DEFAULT 0,
    sent_copy INTEGER,
    smtp_limit INTEGER,
    folded INTEGER NOT NULL DEFAULT 0,
    handled INTEGER NOT NULL DEFAULT 0,
    UNIQUE (address, server)
);
INSERT INTO accounts_new (id, address, server, settings, synced, drafts_up, sent_copy,
        smtp_limit, folded, handled)
    SELECT id, address, json_extract(settings, '$.incoming.host'), settings,
        synced, drafts_up, sent_copy, smtp_limit, folded, handled
    FROM accounts;
DELETE FROM sqlite_sequence WHERE name = 'accounts_new';
INSERT INTO sqlite_sequence (name, seq)
    SELECT 'accounts_new', seq FROM sqlite_sequence WHERE name = 'accounts';
DROP TABLE accounts;
ALTER TABLE accounts_new RENAME TO accounts;
";
