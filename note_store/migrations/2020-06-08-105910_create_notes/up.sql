-- Your SQL goes here

CREATE TABLE notes (
  id uuid DEFAULT uuid_generate_v4() PRIMARY KEY,
  body TEXT NOT NULL,
  "created_at" timestamp without time zone DEFAULT now() NOT NULL,
  "updated_at" timestamp without time zone DEFAULT now() NOT NULL
);
