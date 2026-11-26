-- Schema 0014: the reasoning effort saved for each session. NULL means the model default.

ALTER TABLE sessions ADD COLUMN reasoning_effort TEXT CHECK (reasoning_effort IS NULL OR reasoning_effort IN ('low', 'medium', 'high'));
