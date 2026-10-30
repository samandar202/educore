ALTER TABLE media_files ADD COLUMN storage_backend TEXT NOT NULL DEFAULT 'local' CHECK(storage_backend IN ('local','supabase'));
-- Rust connects as table owner; browser Data API access is denied.
ALTER TABLE public.academies ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.academies FROM PUBLIC;
ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.users FROM PUBLIC;
ALTER TABLE public.study_groups ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.study_groups FROM PUBLIC;
ALTER TABLE public.parent_links ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.parent_links FROM PUBLIC;
ALTER TABLE public.sessions ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.sessions FROM PUBLIC;
ALTER TABLE public.assignments ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.assignments FROM PUBLIC;
ALTER TABLE public.submissions ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.submissions FROM PUBLIC;
ALTER TABLE public.leads ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.leads FROM PUBLIC;
ALTER TABLE public.audit_log ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.audit_log FROM PUBLIC;
ALTER TABLE public.attendance ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.attendance FROM PUBLIC;
ALTER TABLE public.legacy_imports ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.legacy_imports FROM PUBLIC;
ALTER TABLE public.courses ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.courses FROM PUBLIC;
ALTER TABLE public.course_lessons ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.course_lessons FROM PUBLIC;
ALTER TABLE public.media_files ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.media_files FROM PUBLIC;
ALTER TABLE public.submission_versions ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.submission_versions FROM PUBLIC;
ALTER TABLE public.calendar_events ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.calendar_events FROM PUBLIC;
ALTER TABLE public.notifications ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.notifications FROM PUBLIC;
ALTER TABLE public.account_tokens ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.account_tokens FROM PUBLIC;
ALTER TABLE public.mail_jobs ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public.mail_jobs FROM PUBLIC;
ALTER TABLE public._sqlx_migrations ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE public._sqlx_migrations FROM PUBLIC;
DO $$ DECLARE r TEXT; t TEXT; BEGIN
 FOREACH r IN ARRAY ARRAY['anon','authenticated'] LOOP
 IF EXISTS(SELECT 1 FROM pg_roles WHERE rolname=r) THEN
 FOREACH t IN ARRAY ARRAY['academies','users','study_groups','parent_links','sessions','assignments','submissions','leads','audit_log','attendance','legacy_imports','courses','course_lessons','media_files','submission_versions','calendar_events','notifications','account_tokens','mail_jobs','_sqlx_migrations'] LOOP
 EXECUTE format('REVOKE ALL ON TABLE public.%I FROM %I',t,r);
 END LOOP; END IF; END LOOP; END $$;
