-- ========================================
-- Address
-- ========================================
CREATE TABLE "address" (
    "id" UUID PRIMARY KEY,
    "address_text" TEXT NOT NULL,
    "latitude" DOUBLE PRECISION NOT NULL,
    "longitude" DOUBLE PRECISION NOT NULL
);

-- ========================================
-- Point
-- ========================================
CREATE TABLE "point" (
    "id" UUID PRIMARY KEY,
    "address" UUID NOT NULL REFERENCES "address" ("id"),
    "name" TEXT NOT NULL,
    "time" TEXT NOT NULL
);

-- ========================================
-- Planconfig
-- ========================================
CREATE TYPE access as enum(
    'link','account' 
);

CREATE TYPE language as enum(
    'deu','eng' 
);

CREATE TABLE "plan_config" (
    "id" UUID PRIMARY KEY,
    "access" access[] NOT NULL,
    "title" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "date" DATE NOT NULL,
    "language" language NOT NULL
);

-- ========================================
-- Plan
-- ========================================
CREATE TABLE "plan" (
    "id" UUID PRIMARY KEY,
    "data" JSONB NOT NULL,

    -- 1. Validate root structure (data must be an object containing specific keys)
    CONSTRAINT chk_plan_root CHECK (
        jsonb_typeof(data) = 'object' AND
        jsonb_typeof(data->'hosting_list') = 'array' AND
        jsonb_typeof(data->'walking_path') = 'object'
    ),

    -- 2. Validate Hosting elements (Must be objects & contain required fields)
    CONSTRAINT chk_hosting_list_elements CHECK (
        -- Fails if any element is NOT an object or a required field is missing
        NOT jsonb_path_exists(data, '
            $.hosting_list[*] ? (
                @.type() != "object" || 
                !exists(@.id) || 
                !exists(@.name) || 
                !exists(@.host) || 
                !exists(@.guest_list)
            )
        ')
    ),

    -- 3. Validate data types within Hosting (IDs as strings, guest_list as array)
    CONSTRAINT chk_hosting_list_types CHECK (
        NOT jsonb_path_exists(data, '
            $.hosting_list[*] ? (
                @.id.type() != "string" || 
                @.name.type() != "string" || 
                @.host.type() != "string" || 
                @.guest_list.type() != "array"
            )
        ')
    ),

    -- 4. Validate items INSIDE guest_list (Must be strings/UUIDs)
    CONSTRAINT chk_guest_list_items CHECK (
        NOT jsonb_path_exists(data, '$.hosting_list[*].guest_list[*] ? (@.type() != "string")')
    ),

    -- 5. Validate values in walking_path HashMap (Must be arrays)
    CONSTRAINT chk_walking_path_values CHECK (
        NOT jsonb_path_exists(data, '$.walking_path.keyvalue() ? (@.value.type() != "array")')
    ),

    -- 6. Validate items INSIDE walking_path arrays (Must be strings/UUIDs)
    CONSTRAINT chk_walking_path_items CHECK (
        NOT jsonb_path_exists(data, '$.walking_path.*[*] ? (@.type() != "string")')
    )
);

-- ========================================
-- Share Config
-- ========================================
CREATE TYPE team_fields as enum(
    'mail','phone','members','diets'
);

CREATE TABLE "share" (
    "id" UUID PRIMARY KEY,
    "created" TIMESTAMPTZ NOT NULL,
    "invite_text" TEXT NOT NULL,
    "needs_login" BOOLEAN NOT NULL,
    "default_needs_check" BOOLEAN NOT NULL,
    "required_fields" team_fields[] NULL,
    "max_teams" INTEGER NULL,
    "registration_deadline" TIMESTAMPTZ NULL
);

-- ========================================
-- CookAndRun
-- ========================================
CREATE TABLE "cook_and_run" (
    "id" UUID PRIMARY KEY,
    "user_id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "created" TIMESTAMPTZ NOT NULL,
    "edited" TIMESTAMPTZ NOT NULL,
    "occur" TIMESTAMPTZ NOT NULL,
    "start_point" UUID NULL REFERENCES "point" ("id") ON DELETE SET NULL,
    "end_point" UUID NULL REFERENCES "point" ("id") ON DELETE SET NULL,
    "share_team_config" UUID NULL REFERENCES "share" ("id") ON DELETE SET NULL,
    "plan" UUID NULL REFERENCES "plan" ("id") ON DELETE SET NULL,        
    "plan_config" UUID NULL REFERENCES "plan_config" ("id") ON DELETE SET NULL        
);

CREATE INDEX idx_cookandrun_user_id ON "cook_and_run" ("user_id");

-- ========================================
-- Team
-- ========================================
CREATE TABLE "team" (
    "id" UUID PRIMARY KEY,
    "cook_and_run_id" UUID NOT NULL,
    "created_by_user" TEXT NULL,
    "name" TEXT NOT NULL,
    "created" TIMESTAMPTZ NOT NULL,
    "edited" TIMESTAMPTZ NOT NULL,
    "address" UUID NOT NULL,
    "mail" TEXT NULL,
    "phone" TEXT NULL,
    "members" INTEGER NULL,
    "diets" TEXT NULL,
    "needs_check" BOOLEAN NOT NULL,
    FOREIGN KEY ("cook_and_run_id") REFERENCES "cook_and_run" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("address") REFERENCES "address" ("id")
);

CREATE INDEX idx_team_cook_and_run ON "team" ("cook_and_run_id");
CREATE INDEX idx_team_user ON "team" ("created_by_user");

-- ========================================
-- Note
-- ========================================
CREATE TABLE "note" (
    "id" UUID PRIMARY KEY,
    "team_id" UUID NOT NULL,
    "headline" TEXT NOT NULL,
    "content" TEXT NOT NULL,
    "created" TIMESTAMPTZ NOT NULL,
    FOREIGN KEY ("team_id") REFERENCES "team"("id") ON DELETE CASCADE
);

CREATE INDEX idx_note_team_id ON "note" ("team_id");

-- ========================================
-- Course
-- ========================================
CREATE TABLE "course" (
    "id" UUID PRIMARY KEY,
    "cook_and_run_id" UUID NOT NULL REFERENCES "cook_and_run" ("id") ON DELETE CASCADE,
    "name" TEXT NOT NULL,
    "time" TEXT NOT NULL,
    "has_multiple_hosts" BOOLEAN NOT NULL
);

CREATE INDEX idx_course_cook_and_run ON "course" ("cook_and_run_id");

