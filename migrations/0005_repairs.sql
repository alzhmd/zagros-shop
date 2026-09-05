-- فاز ۳: سیستم تعمیرات.

CREATE TABLE repair_requests (
    id             uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticket_number  bigserial UNIQUE,
    user_id        uuid REFERENCES users(id) ON DELETE SET NULL,
    full_name      text NOT NULL,
    phone          text NOT NULL,
    device_type    text NOT NULL,
    brand          text,
    model          text,
    problem        text NOT NULL,
    method         text NOT NULL,                   -- onsite (اعزام به محل) | dropoff (آوردن به انبار)
    province       text,
    city           text,
    address        text,
    status         text NOT NULL DEFAULT 'submitted',
        -- submitted | reviewing | scheduled | in_progress | awaiting_parts | ready | completed | cancelled
    technician_id  uuid REFERENCES users(id) ON DELETE SET NULL,
    cost_estimate  bigint,
    final_cost     bigint,
    admin_note     text,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_repairs_user ON repair_requests(user_id, created_at DESC);
CREATE INDEX idx_repairs_status ON repair_requests(status);
CREATE INDEX idx_repairs_technician ON repair_requests(technician_id);

CREATE TABLE repair_status_log (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    repair_id   uuid NOT NULL REFERENCES repair_requests(id) ON DELETE CASCADE,
    status      text NOT NULL,
    note        text,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_repair_log_repair ON repair_status_log(repair_id, created_at);

CREATE TRIGGER trg_repairs_touch BEFORE UPDATE ON repair_requests
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
