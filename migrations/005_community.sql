-- V6: erkin ro'yxat, o'qituvchi tasdig'i, farzand kodi, o'yin ballari.
ALTER TABLE users ADD COLUMN pending BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE users ADD COLUMN link_code TEXT;
CREATE UNIQUE INDEX users_link_code ON users(academy_id, link_code) WHERE link_code IS NOT NULL;
CREATE TABLE game_scores (
 id BIGSERIAL PRIMARY KEY,
 academy_id BIGINT NOT NULL,
 user_id BIGINT NOT NULL,
 game TEXT NOT NULL CHECK(game IN ('words')),
 score INT NOT NULL CHECK(score BETWEEN 0 AND 200),
 xp INT NOT NULL CHECK(xp BETWEEN 0 AND 100),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 FOREIGN KEY(academy_id,user_id) REFERENCES users(academy_id,id)
);
CREATE INDEX game_scores_user ON game_scores(academy_id,user_id,created_at);
ALTER TABLE public.game_scores ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.game_scores FROM PUBLIC;
DO $$ DECLARE r TEXT; BEGIN
 FOREACH r IN ARRAY ARRAY['anon','authenticated'] LOOP
 IF EXISTS(SELECT 1 FROM pg_roles WHERE rolname=r) THEN
 EXECUTE format('REVOKE ALL ON TABLE public.game_scores FROM %I',r);
 EXECUTE format('REVOKE ALL ON SEQUENCE public.game_scores_id_seq FROM %I',r);
 END IF; END LOOP; END $$;
