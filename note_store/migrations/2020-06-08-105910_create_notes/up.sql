-- Your SQL goes here

CREATE TABLE notes (
  id uuid DEFAULT uuid_generate_v4() NOT NULL,
  body TEXT NOT NULL,
  "createdAt" timestamp without time zone DEFAULT now() NOT NULL,
  "updatedAt" timestamp without time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY notes
  ADD CONSTRAINT "PK_96d0c172a4fba276b1bbed43058" PRIMARY KEY (id);
