-- فاز ارتقا U6: پرسش و پاسخ محصول.

CREATE TABLE product_questions (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    product_id  uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    user_id     uuid REFERENCES users(id) ON DELETE SET NULL,
    author_name text NOT NULL,
    body        text NOT NULL,
    status      text NOT NULL DEFAULT 'pending',   -- pending | approved | rejected
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_questions_product ON product_questions(product_id, status);
CREATE INDEX idx_questions_status ON product_questions(status, created_at DESC);

CREATE TABLE product_answers (
    id          uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    question_id uuid NOT NULL REFERENCES product_questions(id) ON DELETE CASCADE,
    body        text NOT NULL,
    is_staff    boolean NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_answers_question ON product_answers(question_id, created_at);
