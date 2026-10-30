ALTER TABLE academies ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE academies ADD COLUMN contact TEXT NOT NULL DEFAULT '';
CREATE TABLE invoices (
 id BIGSERIAL PRIMARY KEY, academy_id BIGINT NOT NULL REFERENCES academies(id), student_id BIGINT NOT NULL,
 period TEXT NOT NULL CHECK(period ~ '^[0-9]{4}-[0-9]{2}$'), title TEXT NOT NULL,
 amount BIGINT NOT NULL CHECK(amount BETWEEN 1000 AND 100000000), due_date DATE NOT NULL,
 voided BOOLEAN NOT NULL DEFAULT false, created_by BIGINT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(academy_id,id),
 FOREIGN KEY(academy_id,student_id) REFERENCES users(academy_id,id),FOREIGN KEY(academy_id,created_by) REFERENCES users(academy_id,id)
);
CREATE UNIQUE INDEX invoices_active_month ON invoices(academy_id,student_id,period) WHERE voided=false;
CREATE TABLE payments (
 id TEXT PRIMARY KEY,academy_id BIGINT NOT NULL,invoice_id BIGINT NOT NULL,
 amount BIGINT NOT NULL CHECK(amount BETWEEN 1 AND 100000000),method TEXT NOT NULL CHECK(method IN ('cash','transfer','card')),
 note TEXT NOT NULL DEFAULT '',voided_at TIMESTAMPTZ,void_reason TEXT NOT NULL DEFAULT '',recorded_by BIGINT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),FOREIGN KEY(academy_id,invoice_id) REFERENCES invoices(academy_id,id),
 FOREIGN KEY(academy_id,recorded_by) REFERENCES users(academy_id,id)
);
CREATE TABLE ai_drafts (
 id TEXT PRIMARY KEY,academy_id BIGINT NOT NULL REFERENCES academies(id),user_id BIGINT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('lesson','quiz','feedback','speaking')),input_hash TEXT NOT NULL,
 topic TEXT NOT NULL,level TEXT NOT NULL,audience TEXT NOT NULL,duration INT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('running','ready','failed')),title TEXT NOT NULL DEFAULT '',
 student_content TEXT NOT NULL DEFAULT '',teacher_notes TEXT NOT NULL DEFAULT '',error TEXT NOT NULL DEFAULT '',
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),FOREIGN KEY(academy_id,user_id) REFERENCES users(academy_id,id)
);
CREATE INDEX invoices_academy_student ON invoices(academy_id,student_id);
CREATE INDEX payments_invoice ON payments(academy_id,invoice_id);
CREATE INDEX ai_drafts_quota ON ai_drafts(academy_id,created_at);
ALTER TABLE invoices ENABLE ROW LEVEL SECURITY;
ALTER TABLE payments ENABLE ROW LEVEL SECURITY;
ALTER TABLE ai_drafts ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON invoices,payments,ai_drafts FROM PUBLIC;
DO $$ DECLARE r TEXT; BEGIN FOREACH r IN ARRAY ARRAY['anon','authenticated'] LOOP
 IF EXISTS(SELECT 1 FROM pg_roles WHERE rolname=r) THEN EXECUTE format('REVOKE ALL ON invoices,payments,ai_drafts FROM %I',r); END IF;
END LOOP; END $$;
