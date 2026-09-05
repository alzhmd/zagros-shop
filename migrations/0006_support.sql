-- فاز ۴: پشتیبانی — تیکت و چت زنده.

-- تیکت‌های پشتیبانی.
CREATE TABLE tickets (
    id            uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticket_number bigserial UNIQUE,
    user_id       uuid REFERENCES users(id) ON DELETE SET NULL,
    full_name     text NOT NULL,
    email         text,
    subject       text NOT NULL,
    department    text NOT NULL DEFAULT 'general',  -- general | sales | technical | billing
    priority      text NOT NULL DEFAULT 'normal',   -- low | normal | high
    status        text NOT NULL DEFAULT 'open',     -- open | answered | closed
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_tickets_user ON tickets(user_id, created_at DESC);
CREATE INDEX idx_tickets_status ON tickets(status);

-- پیام‌های هر تیکت.
CREATE TABLE ticket_messages (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticket_id   uuid NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    sender      text NOT NULL,                       -- customer | staff
    body        text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_ticket_messages_ticket ON ticket_messages(ticket_id, created_at);

-- جلسات چت زنده.
CREATE TABLE chat_sessions (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     uuid REFERENCES users(id) ON DELETE SET NULL,
    guest_name  text,
    status      text NOT NULL DEFAULT 'active',      -- active | closed
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_chat_sessions_status ON chat_sessions(status, created_at DESC);

-- پیام‌های چت زنده.
CREATE TABLE chat_messages (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id  uuid NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    sender      text NOT NULL,                       -- customer | staff
    body        text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_chat_messages_session ON chat_messages(session_id, created_at);

CREATE TRIGGER trg_tickets_touch BEFORE UPDATE ON tickets
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
