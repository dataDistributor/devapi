-- Create logs table
CREATE TABLE IF NOT EXISTS logs (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    request JSONB NOT NULL,
    response JSONB NOT NULL
);

-- Create credentials table
CREATE TABLE IF NOT EXISTS credentials (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    token TEXT NOT NULL,
    nonce TEXT NOT NULL
);
