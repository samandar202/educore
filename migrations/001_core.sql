CREATE TABLE academies (
 id BIGSERIAL PRIMARY KEY,
 slug TEXT NOT NULL UNIQUE CHECK(slug ~ '^[a-z0-9][a-z0-9-]{2,39}$'),
 name TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE users (
 id BIGSERIAL PRIMARY KEY, academy_id BIGINT NOT NULL REFERENCES academies(id),
 name TEXT NOT NULL, email TEXT NOT NULL, password_hash TEXT NOT NULL,
 role TEXT NOT NULL CHECK(role IN ('owner','teacher','student','parent')),
 group_id BIGINT, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(academy_id,email), UNIQUE(academy_id,id)
);
CREATE TABLE study_groups (
 id BIGSERIAL PRIMARY KEY, academy_id BIGINT NOT NULL REFERENCES academies(id),
 name TEXT NOT NULL, level TEXT NOT NULL, audience TEXT NOT NULL CHECK(audience IN ('child','adult')),
 teacher_id BIGINT NOT NULL, schedule TEXT NOT NULL DEFAULT '',
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(academy_id,id), UNIQUE(academy_id,name),
 FOREIGN KEY(academy_id,teacher_id) REFERENCES users(academy_id,id)
);
ALTER TABLE users ADD FOREIGN KEY(academy_id,group_id) REFERENCES study_groups(academy_id,id);
CREATE TABLE parent_links (
 academy_id BIGINT NOT NULL, parent_id BIGINT NOT NULL, student_id BIGINT NOT NULL,
 PRIMARY KEY(academy_id,parent_id,student_id),
 FOREIGN KEY(academy_id,parent_id) REFERENCES users(academy_id,id),
 FOREIGN KEY(academy_id,student_id) REFERENCES users(academy_id,id)
);
CREATE TABLE sessions (
 token_hash TEXT PRIMARY KEY, user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 expires_at TIMESTAMPTZ NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sessions_user_idx ON sessions(user_id);
CREATE TABLE assignments (
 id BIGSERIAL PRIMARY KEY, academy_id BIGINT NOT NULL, group_id BIGINT NOT NULL,
 title TEXT NOT NULL, instructions TEXT NOT NULL,
 skill TEXT NOT NULL CHECK(skill IN ('grammar','vocabulary','reading','writing','speaking','listening')),
 due_date TEXT NOT NULL DEFAULT '',created_by BIGINT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(academy_id,id),
 FOREIGN KEY(academy_id,group_id) REFERENCES study_groups(academy_id,id),
 FOREIGN KEY(academy_id,created_by) REFERENCES users(academy_id,id)
);
CREATE TABLE submissions (
 id BIGSERIAL PRIMARY KEY, academy_id BIGINT NOT NULL, assignment_id BIGINT NOT NULL, student_id BIGINT NOT NULL,
 answer TEXT NOT NULL, revision INT NOT NULL DEFAULT 1, score INT CHECK(score BETWEEN 0 AND 100), feedback TEXT,
 submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),reviewed_at TIMESTAMPTZ,
 UNIQUE(academy_id,assignment_id,student_id),
 FOREIGN KEY(academy_id,assignment_id) REFERENCES assignments(academy_id,id),
 FOREIGN KEY(academy_id,student_id) REFERENCES users(academy_id,id)
);
CREATE TABLE leads (
 id BIGSERIAL PRIMARY KEY,academy_id BIGINT NOT NULL REFERENCES academies(id),
 name TEXT NOT NULL, phone TEXT NOT NULL, audience TEXT NOT NULL CHECK(audience IN ('child','adult')),
 goal TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'new' CHECK(status IN ('new','contacted','trial','enrolled')),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(academy_id,phone,audience)
);
CREATE TABLE audit_log (
 id BIGSERIAL PRIMARY KEY,academy_id BIGINT NOT NULL REFERENCES academies(id),actor_id BIGINT REFERENCES users(id),
 action TEXT NOT NULL, detail TEXT NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE attendance (
 academy_id BIGINT NOT NULL, group_id BIGINT NOT NULL,student_id BIGINT NOT NULL,lesson_date TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('present','absent','late')),PRIMARY KEY(academy_id,student_id,lesson_date),
 FOREIGN KEY(academy_id,group_id) REFERENCES study_groups(academy_id,id),
 FOREIGN KEY(academy_id,student_id) REFERENCES users(academy_id,id)
);
CREATE TABLE legacy_imports (academy_id BIGINT PRIMARY KEY REFERENCES academies(id), imported_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE INDEX users_academy_idx ON users(academy_id);
CREATE INDEX assignments_group_idx ON assignments(academy_id,group_id);
CREATE INDEX submissions_student_idx ON submissions(academy_id,student_id);
CREATE INDEX audit_academy_idx ON audit_log(academy_id,id DESC);
