-- ========================================
-- Team status (ersetzt needs_check: bool)
-- ========================================
CREATE TYPE team_status AS ENUM (
    'active',
    'review',
    'canceled'
);

ALTER TABLE "team"
    ADD COLUMN "status" team_status NOT NULL DEFAULT 'active';

-- needs_check=true entsprach fachlich "muss geprüft werden" -> review
UPDATE "team" SET "status" = 'review' WHERE "needs_check" = true;

ALTER TABLE "team" DROP COLUMN "needs_check";

-- ========================================
-- Stornierung
-- ========================================
ALTER TABLE "team"
    ADD COLUMN "canceled_at" TIMESTAMPTZ NULL,
    ADD COLUMN "cancel_reason" TEXT NULL;

ALTER TABLE "team" ADD CONSTRAINT chk_team_cancel_consistency CHECK (
    (status = 'canceled' AND canceled_at IS NOT NULL)
    OR (status != 'canceled' AND canceled_at IS NULL AND cancel_reason IS NULL)
);

-- ========================================
-- Self-Service-Zugriff (Deeplink-Token statt Keycloak-Login)
-- ========================================
-- Angenommen: Tabelle enthält aktuell keine Bestandsdaten (Dev-Stand).
-- Falls doch: vor dem NOT NULL erst per UPDATE befüllen.
ALTER TABLE "team"
    ADD COLUMN "access_token" TEXT NOT NULL,
    ADD COLUMN "email_verified_at" TIMESTAMPTZ NULL,
    ADD COLUMN "verification_resend_count" INTEGER NOT NULL DEFAULT 0;

ALTER TABLE "team" ADD CONSTRAINT uq_team_access_token UNIQUE ("access_token");

-- ========================================
-- Share: needs_login -> require_email_verification
-- ========================================
ALTER TABLE "share" RENAME COLUMN "needs_login" TO "require_email_verification";
-- share.default_needs_check bleibt unverändert: steuert weiterhin, ob neue
-- Teams mit status='review' statt 'active' starten (nur die Zielgröße ändert
-- sich von bool auf enum).

ALTER TABLE "share" ADD COLUMN "edit_deadline" TIMESTAMPTZ NULL;

CREATE TYPE audit_actor_type AS ENUM (
    'admin',
    'participant',
    'system'
);

CREATE TYPE audit_action AS ENUM (
    'created',
    'updated',
    'canceled'
);

CREATE TABLE "team_audit_log" (
    "id" UUID PRIMARY KEY,
    "team_id" UUID NOT NULL REFERENCES "team" ("id") ON DELETE CASCADE,
    "actor_type" audit_actor_type NOT NULL,
    "actor_label" TEXT NULL,
    "action" audit_action NOT NULL,
    "changes" JSONB NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_team_audit_log_team_id ON "team_audit_log" ("team_id");
CREATE INDEX idx_team_audit_log_created_at ON "team_audit_log" ("created_at");

ALTER TABLE "share" ADD COLUMN "review_trigger_fields" team_fields[] NULL;

ALTER TYPE audit_action ADD VALUE 'plan_invalidated';

ALTER TABLE "plan" ADD COLUMN "stale_at" TIMESTAMPTZ NULL;

ALTER TABLE "share" ADD COLUMN "notify_admin_on_review" BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE "cook_and_run" ADD COLUMN "admin_notification_email" TEXT NULL;

CREATE TYPE email_type AS ENUM (
    'invitation',
    'route_update',
    'review_notification'
);

CREATE TYPE email_status AS ENUM (
    'pending',
    'sent',
    'failed'
);

CREATE TABLE "email_outbox" (
    "id" UUID PRIMARY KEY,
    "team_id" UUID NULL REFERENCES "team" ("id") ON DELETE CASCADE,
    "recipient_email" TEXT NOT NULL,
    "email_type" email_type NOT NULL,
    "context" JSONB NOT NULL,
    "status" email_status NOT NULL DEFAULT 'pending',
    "attempts" INTEGER NOT NULL DEFAULT 0,
    "last_error" TEXT NULL,
    "next_attempt_at" TIMESTAMPTZ NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "sent_at" TIMESTAMPTZ NULL
);

CREATE INDEX idx_email_outbox_pending ON "email_outbox" ("next_attempt_at")
    WHERE status = 'pending';
CREATE INDEX idx_email_outbox_team_id ON "email_outbox" ("team_id");

ALTER TYPE email_type RENAME VALUE 'review_notification' TO 'admin_notification';

ALTER TABLE "team" ADD COLUMN "last_route_hash" TEXT NULL;