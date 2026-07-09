-- ========================================
-- Drop Indices
-- ========================================
DROP INDEX IF EXISTS idx_cookandrun_user_id;
DROP INDEX IF EXISTS idx_hosting_plan_id;
DROP INDEX IF EXISTS idx_hosting_team_id;
DROP INDEX IF EXISTS idx_hosting_course_id;
DROP INDEX IF EXISTS idx_course_cook_and_run;
DROP INDEX IF EXISTS idx_note_team_id;
DROP INDEX IF EXISTS idx_team_user;
DROP INDEX IF EXISTS idx_team_cook_and_run;

-- ========================================
-- Drop Tables
-- ========================================
DROP TABLE IF EXISTS "CookAndRun" CASCADE;
DROP TABLE IF EXISTS "Share" CASCADE;
DROP TABLE IF EXISTS "Plan" CASCADE;
DROP TABLE IF EXISTS "Hosting" CASCADE;
DROP TABLE IF EXISTS "Course" CASCADE;
DROP TABLE IF EXISTS "Note" CASCADE;
DROP TABLE IF EXISTS "Team" CASCADE;
DROP TABLE IF EXISTS "Address" CASCADE;

-- ========================================
-- Drop Types
-- ========================================
DROP TYPE IF EXISTS team_fields;
