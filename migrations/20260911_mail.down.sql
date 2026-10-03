ALTER TABLE "share" RENAME COLUMN "require_email_verification" TO "needs_login";

ALTER TABLE "team" DROP CONSTRAINT uq_team_access_token;
ALTER TABLE "team" DROP COLUMN "verification_resend_count";
ALTER TABLE "team" DROP COLUMN "email_verified_at";
ALTER TABLE "team" DROP COLUMN "access_token";

ALTER TABLE "team" DROP CONSTRAINT chk_team_cancel_consistency;
ALTER TABLE "team" DROP COLUMN "cancel_reason";
ALTER TABLE "team" DROP COLUMN "canceled_at";

ALTER TABLE "team" ADD COLUMN "needs_check" BOOLEAN NOT NULL DEFAULT false;
UPDATE "team" SET "needs_check" = true WHERE "status" = 'review';
ALTER TABLE "team" DROP COLUMN "status";

DROP TYPE team_status;

ALTER TABLE "share" DROP COLUMN "edit_deadline";

DROP TABLE "team_audit_log";
DROP TYPE audit_action;
DROP TYPE audit_actor_type;

ALTER TABLE "share" DROP COLUMN "review_trigger_fields";

-- Enum-Werte lassen sich in Postgres nicht direkt entfernen; Typ muss neu
-- aufgebaut werden. Schlägt fehl, falls bereits 'plan_invalidated'-Zeilen
-- existieren (das ist bei einem Down-Migration-Aufruf erwartbar/akzeptabel).
ALTER TYPE audit_action RENAME TO audit_action_old;

CREATE TYPE audit_action AS ENUM ('created', 'updated', 'canceled');

ALTER TABLE team_audit_log
    ALTER COLUMN action TYPE audit_action USING action::text::audit_action;

DROP TYPE audit_action_old;

ALTER TABLE "plan" DROP COLUMN "stale_at";

ALTER TABLE "cook_and_run" DROP COLUMN "admin_notification_email";
ALTER TABLE "share" DROP COLUMN "notify_admin_on_review";

DROP TABLE "email_outbox";
DROP TYPE email_status;
DROP TYPE email_type;

ALTER TYPE email_type RENAME VALUE 'admin_notification' TO 'review_notification';

ALTER TABLE "team" DROP COLUMN "last_route_hash";