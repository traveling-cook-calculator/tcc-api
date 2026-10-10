-- =====================================================================
-- Schema für das Rust-Backend (sqlx / PostgreSQL)
-- =====================================================================
BEGIN;
    -- ---------------------------------------------------------------------
    -- ENUM-Typen
    -- ---------------------------------------------------------------------
    CREATE TYPE access AS ENUM ('link', 'account');
    CREATE TYPE language AS ENUM ('deutsch', 'english');
    CREATE TYPE required_field AS ENUM ('mail', 'phone', 'members', 'diets');
    CREATE TYPE audit_actor_type AS ENUM ('admin', 'participant');
    CREATE TYPE audit_action AS ENUM ('created', 'updated', 'canceled', 'plan_invalidated', 'resend_verification_mail');
    CREATE TYPE team_status AS ENUM ('active', 'review','canceled');
    -- ---------------------------------------------------------------------
    -- address
    -- ---------------------------------------------------------------------
    CREATE TABLE address
        (
            id           uuid PRIMARY KEY                                             ,
            address_text text NOT NULL                                                ,
            latitude     double precision NOT NULL CHECK (latitude BETWEEN -90 AND 90),
            longitude    double precision NOT NULL CHECK (longitude BETWEEN -180 AND 180)
        )
    ;
    -- ---------------------------------------------------------------------
    -- point
    -- ---------------------------------------------------------------------
    CREATE TABLE point
        (
            id      uuid PRIMARY KEY                     ,
            address uuid NOT NULL REFERENCES address (id),
            name    text NOT NULL                        ,
                    time text NOT NULL
        )
    ;
    CREATE INDEX idx_point_address
    ON point
        (
            address
        )
    ;
    -- ---------------------------------------------------------------------
    -- project
    -- share_team_config, plan und plan_config werden erst unten per ALTER
    -- mit Foreign Keys versehen, da es zirkuläre Abhängigkeiten gibt.
    -- ---------------------------------------------------------------------
    CREATE TABLE project
        (
            id                       uuid PRIMARY KEY          ,
            user_id                  text NOT NULL             ,
            name                     text NOT NULL             ,
            created                  timestamptz NOT NULL      ,
            edited                   timestamptz NOT NULL      ,
            occur                    timestamptz NOT NULL      ,
            start_point              uuid REFERENCES point (id),
            end_point                uuid REFERENCES point (id),
            admin_notification_email text
        )
    ;
    CREATE INDEX idx_project_user_created
    ON project
        (
            user_id,
            created
        )
    ;
    CREATE INDEX idx_project_user_name
    ON project
        (
            user_id,
            name
        )
    ;
    CREATE INDEX idx_project_user_edited
    ON project
        (
            user_id,
            edited
        )
    ;
    CREATE INDEX idx_project_start_point
    ON project
        (
            start_point
        )
    ;
    CREATE INDEX idx_project_end_point
    ON project
        (
            end_point
        )
    ;
    -- ---------------------------------------------------------------------
    -- share  (id == project.id, siehe ShareRepository::insert)
    -- ---------------------------------------------------------------------
    CREATE TABLE share
        (
            id                         uuid PRIMARY KEY REFERENCES project (id),
            created                    timestamptz NOT NULL                    ,
            invite_text                text NOT NULL                           ,
            require_email_verification boolean NOT NULL DEFAULT false          ,
            default_needs_check        boolean NOT NULL DEFAULT false          ,
            required_fields required_field[]                                   ,
            max_teams             integer CHECK (max_teams BETWEEN 0 AND 255)  ,
            registration_deadline timestamptz                                  ,
            edit_deadline         timestamptz                                  ,
            review_trigger_fields required_field[]                             ,
            notify_admin_on_review boolean NOT NULL DEFAULT false              ,
            notify_admin_on_create boolean NOT NULL DEFAULT false              ,
            notify_admin_on_cancel boolean NOT NULL DEFAULT false
        )
    ;
    -- ---------------------------------------------------------------------
    -- plan  (id == project.id, Hostings und Walking-Path inline in jsonb)
    -- ---------------------------------------------------------------------
    CREATE TABLE plan
        (
            id       uuid PRIMARY KEY REFERENCES project (id),
            data     jsonb NOT NULL                          ,
            stale_at timestamptz
        )
    ;
    -- Für einen Hintergrundjob, der veraltete Pläne sucht
    CREATE INDEX idx_plan_stale_at
    ON plan
        (
            stale_at
        )
    WHERE stale_at IS NOT NULL;
    -- ---------------------------------------------------------------------
    -- plan_config  (id == project.id, siehe PlanConfigRepository::upsert)
    -- ---------------------------------------------------------------------
    CREATE TABLE plan_config
        (
            id uuid PRIMARY KEY REFERENCES project (id),
            access access[] NOT NULL DEFAULT '{}'      ,
            title       text NOT NULL                  ,
            description text NOT NULL                  ,
                        date date NOT NULL             ,
            language language NOT NULL
        )
    ;
    -- ---------------------------------------------------------------------
    -- team
    -- ---------------------------------------------------------------------
    CREATE TABLE team
        (
            id              uuid PRIMARY KEY                                                      ,
            project_id      uuid NOT NULL REFERENCES project (id)                                 ,
            created_by_user text                                                                  ,
            name            text NOT NULL                                                         ,
            created         timestamptz NOT NULL                                                  ,
            edited          timestamptz NOT NULL                                                  ,
            address         uuid NOT NULL REFERENCES address (id)                                 ,
            mail            text                                                                  ,
            phone           text                                                                  ,
            members         integer CHECK (members BETWEEN 0 AND 255)                             ,
            diets           text                                                                  ,
            status team_status NOT NULL                                                           ,
            canceled_at               timestamptz                                                 ,
            cancel_reason             text                                                        ,
            access_token              text NOT NULL                                               ,
            email_verified_at         timestamptz                                                 ,
            verification_resend_count integer CHECK (verification_resend_count BETWEEN 0 AND 255) ,
            last_route_hash           text
        )
    ;
    CREATE INDEX idx_team_project_id
    ON team
        (
            project_id
        )
    ;
    -- ---------------------------------------------------------------------
    -- course
    -- ---------------------------------------------------------------------
    CREATE TABLE course
        (
            id         uuid PRIMARY KEY                     ,
            project_id uuid NOT NULL REFERENCES project (id),
            name       text NOT NULL                        ,
                       time text NOT NULL
        )
    ;
    CREATE INDEX idx_course_project_time
    ON course
        (
            project_id,
            time
        )
    ;
    -- ---------------------------------------------------------------------
    -- note
    -- ---------------------------------------------------------------------
    CREATE TABLE note
        (
            id       uuid PRIMARY KEY                  ,
            team_id  uuid NOT NULL REFERENCES team (id),
            headline text NOT NULL                     ,
            content  text NOT NULL                     ,
            created  timestamptz NOT NULL
        )
    ;
    CREATE INDEX idx_note_team_created
    ON note
        (
            team_id,
            created
        )
    ;
    -- ---------------------------------------------------------------------
    -- team_audit_log
    -- team_id bewusst ohne FK: Audit-Einträge sollen das Löschen eines
    -- Teams überleben.
    -- ---------------------------------------------------------------------
    CREATE TABLE team_audit_log
        (
            id      uuid PRIMARY KEY            ,
            team_id uuid NOT NULL               ,
            actor_type audit_actor_type NOT NULL,
            actor_label text                    ,
            action audit_action NOT NULL        ,
            changes    jsonb NOT NULL           ,
            created_at timestamptz NOT NULL
        )
    ;
    CREATE INDEX idx_team_audit_log_team_created
    ON team_audit_log
        (
            team_id,
            created_at DESC
        )
    ;
    COMMIT;