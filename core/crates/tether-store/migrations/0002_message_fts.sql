-- FTS5 text parts. The message row remains the source of truth.
CREATE VIRTUAL TABLE message_fts USING fts5(
    part_id UNINDEXED, message_id UNINDEXED, session_id UNINDEXED, body,
    tokenize = 'unicode61'
);

INSERT INTO message_fts (rowid, part_id, message_id, session_id, body)
SELECT p.rowid, p.id, m.id, m.session_id, json_extract(p.body_json, '$.text')
FROM message_parts p JOIN messages m ON m.id = p.message_id
WHERE p.kind = 'text' AND m.rewound = 0;

CREATE TRIGGER message_fts_part_insert AFTER INSERT ON message_parts BEGIN
    INSERT INTO message_fts (rowid, part_id, message_id, session_id, body)
    SELECT NEW.rowid, NEW.id, m.id, m.session_id, json_extract(NEW.body_json, '$.text')
    FROM messages m WHERE m.id = NEW.message_id AND NEW.kind = 'text' AND m.rewound = 0;
END;

CREATE TRIGGER message_fts_part_update AFTER UPDATE ON message_parts BEGIN
    DELETE FROM message_fts WHERE rowid = OLD.rowid;
    INSERT INTO message_fts (rowid, part_id, message_id, session_id, body)
    SELECT NEW.rowid, NEW.id, m.id, m.session_id, json_extract(NEW.body_json, '$.text')
    FROM messages m WHERE m.id = NEW.message_id AND NEW.kind = 'text' AND m.rewound = 0;
END;

CREATE TRIGGER message_fts_part_delete AFTER DELETE ON message_parts BEGIN
    DELETE FROM message_fts WHERE rowid = OLD.rowid;
END;

CREATE TRIGGER message_fts_rewind AFTER UPDATE OF rewound ON messages BEGIN
    DELETE FROM message_fts WHERE rowid IN (SELECT rowid FROM message_parts WHERE message_id = NEW.id);
    INSERT INTO message_fts (rowid, part_id, message_id, session_id, body)
    SELECT p.rowid, p.id, NEW.id, NEW.session_id, json_extract(p.body_json, '$.text')
    FROM message_parts p WHERE p.message_id = NEW.id AND p.kind = 'text' AND NEW.rewound = 0;
END;

CREATE TRIGGER message_fts_message_delete BEFORE DELETE ON messages BEGIN
    DELETE FROM message_fts WHERE rowid IN (SELECT rowid FROM message_parts WHERE message_id = OLD.id);
END;
